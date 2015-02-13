use async::Future;
use std::sync::Arc;
use core_local::task_queue::{self, Thunk};

pub fn new_pair<T: Send>() -> (FutureValue<T>, FutureValueSetter<T>) {
    FutureValueInner::new()
}

pub struct FutureValue<T: Send> {
    inner: Arc<FutureValueInner<T>>,
}

impl<T: Send> FutureValue<T> {
    pub fn new<F>(f: F) -> FutureValue<T> where F: FnOnce() -> T + Send {
        let (future, setter) = FutureValueInner::new();
        let task = Thunk::new(move || setter.set(f()));
        assert!(task_queue::add(task).is_ok());
        future
    }

    pub fn from_value(value: T) -> FutureValue<T> {
        let (future, setter) = FutureValueInner::new();
        setter.set(value);
        future
    }
}

impl<T: Send> Future for FutureValue<T> {
    type Item = T;

    fn then<F>(self, f: F) where F: FnOnce(<Self as Future>::Item) + Send {
        unsafe{self.inner.set_then(Thunk::with_arg(f))}
    }
}

pub struct FutureValueSetter<T: Send> {
    inner: Arc<FutureValueInner<T>>,    
}

impl<T: Send> FutureValueSetter<T> {
    pub fn set(self, value: T) {
        unsafe{self.inner.set_value(value)}
    }
}

struct FutureValueInner<T: Send> {
    value: Option<T>,
    then: Option<Thunk<T,()>>,    
}

impl<T: Send> FutureValueInner<T> {
    fn new() -> (FutureValue<T>, FutureValueSetter<T>) {
        let inner = Arc::new(FutureValueInner::<T> {
            value: None,
            then: None, 
        });
        let future_value = FutureValue{inner: inner.clone()};
        let setter = FutureValueSetter{inner: inner};
        (future_value, setter)
    }

    unsafe fn set_then(&self, then: Thunk<T,()>) {
        (&mut *self.as_mut()).then = Some(then);
    }    

    unsafe fn set_value(&self, value: T) {
        (&mut *self.as_mut()).value = Some(value);
    }

    fn as_mut(&self) -> *mut FutureValueInner<T> {
        self as *const _ as *mut _
    }
}

unsafe impl<T: Send> Sync for FutureValueInner<T> {} 

#[unsafe_destructor]
impl<T: Send> Drop for FutureValueInner<T> {
    fn drop(&mut self) {
        if let (Some(value), Some(then)) = (self.value.take(), self.then.take()) {
            let task = Thunk::new(move || then.invoke(value));
            assert!(task_queue::add(task).is_ok())
        }
    }
}
