use std::mem;
use std::ptr::Unique;
use std::sync::atomic::{AtomicBool, Ordering};
use core_local::task_queue::{self, Thunk};

pub trait Future: Send {
    type Item: Send;

    fn then<F>(self, f: F) where F: FnOnce(Self::Item) + Send;
}

pub trait FutureExt: Future + Sized {
    fn map<F, B>(self, f: F) -> Map<Self, F> where 
        F: FnOnce(Self::Item) -> B + Send,
    {
        Map{future: self, f: f}
    }
}

impl<Fut> FutureExt for Fut where Fut: Future {}

#[must_use = "future adaptors are lazy and do nothing unless consumed"]
pub struct Map<Fut, F> {
    future: Fut,
    f: F,
}

impl<Fut: Future, F, B: Send> Future for Map<Fut, F> where F: FnOnce(Fut::Item) -> B + Send {
    type Item = B;

    fn then<G>(self, g: G) where G: FnOnce(<Self as Future>::Item) + Send {
        let Map{future, f} = self;
        future.then(move |b| g(f(b)))
    }
}


pub struct Computation<T: Send> {
    inner: ComputationInnerPointer<T>,
}

struct ComputationResultSetter<T: Send> {
    inner: ComputationInnerPointer<T>,    
}

struct ComputationInnerPointer<T: Send>(Unique<ComputationInner<T>>);

struct ComputationInner<T: Send> {
    counterpart_finished: AtomicBool,
    value: Option<T>,
    then: Option<Thunk<T,()>>,    
}

impl<T: Send> Computation<T> {
    pub fn new<F>(f: F) -> Computation<T> where F: FnOnce() -> T + Send {
        let (future, setter) = ComputationInner::new();
        let task = Thunk::new(move |:| setter.set(f()));
        assert!(task_queue::add(task).is_ok());
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
    fn set(self, value: T) {
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
                let task = Thunk::new(move |:| then.invoke(value));
                assert!(task_queue::add(task).is_ok())
            }
            let inner: Box<ComputationInner<T>> = unsafe{mem::transmute(self as *mut _)};
            drop(inner);
        }
    }
}
