use std::mem;
use std::ptr::Unique;
use std::sync::atomic::{AtomicBool, Ordering};
use core_local::task_queue::{self, Thunk};

pub struct Future<T: Send> {
    inner: FutureInnerPointer<T>,
}

pub struct FutureSetter<T: Send> {
    inner: FutureInnerPointer<T>,
}

struct FutureInnerPointer<T: Send>(Unique<FutureInner<T>>);

struct FutureInner<T: Send> {
    counterpart_finished: AtomicBool,
    value: Option<T>,
    then: Option<Thunk<T, ()>>,
}

impl<T: Send> Future<T> {

    fn new() -> (Future<T>, FutureSetter<T>) {
        FutureInner::new()
    }

    pub fn from_fn<F>(f: F) -> Future<T> where F: FnOnce()->T, F: Send {
        let (future, setter) = Future::new();
        let task = Thunk::new(move |:| setter.set(f()));
        assert!(task_queue::add(task).is_ok());
        future
    }

    pub fn then<F, V>(self, f: F) -> Future<V> where V:Send, F: FnOnce(T)->V + Send {
        let (future, future_setter) = Future::new();
        let then = move |: value| future_setter.set(f(value));

        unsafe{self.inner.set_then(Thunk::with_arg(then))};
        future
    }

    pub fn get(self) -> T {
        let inner = unsafe{&mut *(self.inner.0).0};
        while !inner.counterpart_finished.load(Ordering::Relaxed) {
            // busy wait...
        }
        match inner.value.take() {
            None => unreachable!(),
            Some(v) => v,
        }
    }
}

impl<T: Send> FutureSetter<T> {
    pub fn set(self, value: T) {
        unsafe{self.inner.set_value(value)};
    }
}

impl<T: Send> FutureInnerPointer<T> {
    unsafe fn set_then(self, then: Thunk<T,()>) {
        self.as_ref().then = Some(then);
    }    

    unsafe fn set_value(self, value: T) {
        self.as_ref().value = Some(value);
    }

    unsafe fn as_ref(&self) -> &mut FutureInner<T> {
        &mut *(self.0).0
    }    
}

#[unsafe_destructor]
impl<T: Send> Drop for FutureInnerPointer<T> {
    fn drop(&mut self) {
        unsafe{self.as_ref().invoke_if_set()}
    }
}

impl<T: Send> FutureInner<T> {
    fn new() -> (Future<T>, FutureSetter<T>) {
        let inner = FutureInner::<T> {
            value: None,
            then: None, 
            counterpart_finished: AtomicBool::new(false),
        };
        let inner_ptr = unsafe{mem::transmute(Box::new(inner))};
        (Future{inner: FutureInnerPointer(Unique(inner_ptr))}, 
            FutureSetter{inner: FutureInnerPointer(Unique(inner_ptr))})
    }

    fn invoke_if_set(&mut self) {
        if self.counterpart_finished.compare_and_swap(false, true, Ordering::SeqCst) == true {
            if let (Some(value), Some(then)) = (self.value.take(), self.then.take()) {
                let task = Thunk::new(move |:| then.invoke(value));
                assert!(task_queue::add(task).is_ok())
            }
            let inner: Box<FutureInner<T>> = unsafe{mem::transmute(self as *mut _)};
            drop(inner);
        }
    }
}

#[test]
fn test_set_first() {
    let (future, setter) = Future::new();

    setter.set(42);
    let f = move |: x: i32| {
        assert_eq!(x, 42);
        4
    };

    let result = future.then(f);
    assert_eq!(result.get(), 4);
}

#[test]
fn test_then_first() {
    let (future, setter) = Future::new();

    let f = move |: x: i32| {
        assert_eq!(x, 42);
        4
    };
    let result = future.then(f);

    setter.set(42);

    assert_eq!(result.get(), 4);
}

#[test]
fn test_chain() {
    fn test(x: i32) -> i32 {
        x/2 + 2
    }
    let (result1, setter) = Future::new();

    let result2 = result1.then(test); // 10
    let result3 = result2.then(test); // 7
    let result4 = result3.then(test); // 5

    setter.set(16);

    assert_eq!(result4.get(), 5);
}
