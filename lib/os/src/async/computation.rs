use async::Future;
use std::mem;
use std::ptr::Unique;
use std::sync::atomic::{AtomicBool, Ordering};
use core_local::task_queue::{self, Thunk};

pub struct Computation<T: Send> {
    inner: ComputationInnerPointer<T>,
}

pub struct ComputationResultSetter<T: Send> {
    inner: ComputationInnerPointer<T>,    
}

struct ComputationInnerPointer<T: Send>(Unique<ComputationInner<T>>);

struct ComputationInner<T: Send> {
    counterpart_finished: AtomicBool,
    value: Option<T>,
    then: Option<Thunk<T,()>>,    
}

pub fn new_pair<T: Send>() -> (Computation<T>, ComputationResultSetter<T>) {
    ComputationInner::new()
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

impl<T: Send> ComputationResultSetter<T> {
    pub fn set(self, value: T) {
        unsafe{self.inner.set_value(value)}
    }
}

impl<T: Send> ComputationInnerPointer<T> {
    unsafe fn set_then(self, then: Thunk<T,()>) {
        self.as_ref().then = Some(then); // + implicitely invoke destructor
    }    

    unsafe fn set_value(self, value: T) {
        self.as_ref().value = Some(value); // + implicitely invoke destructor
    }

    unsafe fn as_ref(&self) -> &mut ComputationInner<T> {
        &mut *(self.0).0
    }
}

#[unsafe_destructor]
impl<T: Send> Drop for ComputationInnerPointer<T> {
    fn drop(&mut self) {
        unsafe{self.as_ref().invoke_if_set()}
    }
}

impl<T: Send> ComputationInner<T> {
    fn new() -> (Computation<T>, ComputationResultSetter<T>) {
        let inner = ComputationInner::<T> {
            value: None,
            then: None, 
            counterpart_finished: AtomicBool::new(false),
        };
        let inner_ptr = unsafe{mem::transmute(Box::new(inner))};
        let computation = Computation{inner: ComputationInnerPointer(Unique(inner_ptr))};
        let setter = ComputationResultSetter{inner: ComputationInnerPointer(Unique(inner_ptr))};
        (computation, setter)
    }

    unsafe fn invoke_if_set(&mut self) {
        if self.counterpart_finished.compare_and_swap(false, true, Ordering::SeqCst) == true {
            if let (Some(value), Some(then)) = (self.value.take(), self.then.take()) {
                let task = Thunk::new(move || then.invoke(value));
                assert!(task_queue::add(task).is_ok())
            }
            let inner: Box<ComputationInner<T>> = unsafe{mem::transmute(self as *mut _)};
            drop(inner);
        }
    }
}
