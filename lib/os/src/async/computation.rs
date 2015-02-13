use async::Future;
use std::sync::Arc;
use core_local::task_queue::{self, Thunk};

pub fn new_pair<T: Send>() -> (Computation<T>, ComputationResultSetter<T>) {
    ComputationInner::new()
}

pub struct Computation<T: Send> {
    inner: Arc<ComputationInner<T>>,
}

impl<T: Send> Computation<T> {
    pub fn new<F>(f: F) -> Computation<T> where F: FnOnce() -> T + Send {
        let (future, setter) = ComputationInner::new();
        let task = Thunk::new(move || setter.set(f()));
        assert!(task_queue::add(task).is_ok());
        future
    }

    pub fn from_value(value: T) -> Computation<T> {
        let (future, setter) = ComputationInner::new();
        setter.set(value);
        future
    }
}

impl<T: Send> Future for Computation<T> {
    type Item = T;

    fn then<F>(self, f: F) where F: FnOnce(<Self as Future>::Item) + Send {
        unsafe{self.inner.set_then(Thunk::with_arg(f))}
    }
}

pub struct ComputationResultSetter<T: Send> {
    inner: Arc<ComputationInner<T>>,    
}

impl<T: Send> ComputationResultSetter<T> {
    pub fn set(self, value: T) {
        unsafe{self.inner.set_value(value)}
    }
}

struct ComputationInner<T: Send> {
    value: Option<T>,
    then: Option<Thunk<T,()>>,    
}

impl<T: Send> ComputationInner<T> {
    fn new() -> (Computation<T>, ComputationResultSetter<T>) {
        let inner = Arc::new(ComputationInner::<T> {
            value: None,
            then: None, 
        });
        let computation = Computation{inner: inner.clone()};
        let setter = ComputationResultSetter{inner: inner};
        (computation, setter)
    }

    unsafe fn set_then(&self, then: Thunk<T,()>) {
        (&mut *self.as_mut()).then = Some(then);
    }    

    unsafe fn set_value(&self, value: T) {
        (&mut *self.as_mut()).value = Some(value);
    }

    fn as_mut(&self) -> *mut ComputationInner<T> {
        self as *const _ as *mut _
    }
}

unsafe impl<T: Send> Sync for ComputationInner<T> {} 

#[unsafe_destructor]
impl<T: Send> Drop for ComputationInner<T> {
    fn drop(&mut self) {
        if let (Some(value), Some(then)) = (self.value.take(), self.then.take()) {
            let task = Thunk::new(move || then.invoke(value));
            assert!(task_queue::add(task).is_ok())
        }
    }
}
