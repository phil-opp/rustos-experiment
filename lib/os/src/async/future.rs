use std::mem;
use std::ptr::Unique;
use std::sync::atomic::{AtomicBool, Ordering};
use core_local::task_queue::{self, Thunk};

pub struct Future<T: Send> {
    inner: Unique<FutureInner<T>>,
}

pub struct FutureSetter<T: Send> {
    inner: Unique<FutureInner<T>>,
}

struct FutureInner<T: Send> {
    counterpart_finished: AtomicBool,
    value: Option<T>,
    then: Option<Thunk<T, ()>>,
}

impl<T: Send> Future<T> {

    pub fn from_fn<F>(f: F) -> Future<T> where F: FnOnce()->T, F: Send {
        let (future, setter) = Future::new();
        let task = Thunk::new(move |:| setter.set(f()));
        assert!(task_queue::add(task).is_ok());
        future
    }

    fn new() -> (Future<T>, FutureSetter<T>) {
        let inner = FutureInner::<T> {
            value: None,
            then: None, 
            counterpart_finished: AtomicBool::new(false),
        };
        let inner_ptr = unsafe{mem::transmute(Box::new(inner))};
        (Future{inner: Unique(inner_ptr)}, FutureSetter{inner: Unique(inner_ptr)}) 
    }

    pub fn then<F, V>(self, f: F) -> Future<V> where V:Send, F: FnOnce(T)->V + Send {
        let (future, future_setter) = Future::new();
        let then = move |: value| future_setter.set(f(value));

        let inner = unsafe{self.get_inner()};
        inner.then = Some(Thunk::with_arg(then));
        inner.invoke_if_set();
        future
    }

    pub fn get(self) -> T {
        let inner = unsafe{self.get_inner()};
        while !inner.counterpart_finished.load(Ordering::Relaxed) {
            // busy wait...
        }
        match inner.value.take() {
            None => unreachable!(),
            Some(v) => v,
        }
    }

    unsafe fn get_inner(&self) -> &mut FutureInner<T> {
        &mut *self.inner.0
    }
}

impl<T: Send> FutureSetter<T> {
    pub fn set(self, value: T) {
        let inner = unsafe{self.get_inner()};
        inner.value = Some(value);
        inner.invoke_if_set();
    }

    unsafe fn get_inner(&self) -> &mut FutureInner<T> {
        &mut *self.inner.0
    }
}

impl<T: Send> FutureInner<T> {
    fn invoke_if_set(&mut self) {
        if self.counterpart_finished.compare_and_swap(false, true, Ordering::SeqCst) {
            if let (Some(value), Some(then)) = (self.value.take(), self.then.take()) {
                let task = Thunk::new(move |:| then.invoke(value));
                assert!(task_queue::add(task).is_ok())
            } else {
                unreachable!();
            }
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
