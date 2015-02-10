use std::mem;
use std::ptr::Unique;
use std::sync::atomic::{AtomicBool, Ordering};
use super::spsc_queue::Queue;
use core_local::task_queue::{self, Thunk};
use async;

pub struct Stream<T: Send> {
    inner: StreamInnerPointer<T>,
}

pub struct StreamSender<T: Send> {
    inner: StreamInnerPointer<T>,
}

pub struct StreamInnerPointer<T: Send>(Unique<StreamInner<T>>); 

pub struct StreamInner<T: Send> {
    queue: Queue<T>,
    foreach: Option<MutThunk<T>>,
    counterpart_finished: AtomicBool,
}

impl<T: Send> StreamInner<T> {
    fn new() -> (Stream<T>, StreamSender<T>) {
        let inner: StreamInner<T> = StreamInner {
            queue: unsafe{Queue::new(0)},
            foreach: None,
            counterpart_finished: AtomicBool::new(false),
        };
        let inner_ptr = unsafe{mem::transmute(Box::new(inner))};
        (Stream {
            inner: StreamInnerPointer(Unique(inner_ptr)),
        }, StreamSender{inner: StreamInnerPointer(Unique(inner_ptr))})
    }
}

impl<T: Send> StreamInnerPointer<T> {

    /// must only be called by StreamSender    
    unsafe fn send(&mut self, value: T) {
        if self.as_ref().counterpart_finished.load(Ordering::Relaxed) == true {
            unimplemented!();
        } else {
            self.as_ref().queue.push(value)
        }
    }

    /// must only be called by StreamSender
    unsafe fn close(self) {
        if self.as_ref().counterpart_finished.compare_and_swap(false, true, Ordering::Relaxed) {
            // receiver was dropped -> invoke remaining tasks
            let mut inner: Box<StreamInner<T>> = mem::transmute(self);
            if let Some(mut f) = inner.foreach.take() {
                let task = Thunk::new(move || {
                    while let Some(value) = inner.queue.pop() {
                        f.invoke(value)
                    }
                });
                assert!(task_queue::add(task).is_ok());
            }
        }
    }

    /// must only be called by Stream (Receiver)
    unsafe fn set_foreach<F>(self, f: F) where F: FnMut(T) + Send {
        let foreach = MutThunk::with_arg(f);
        self.as_ref().foreach = Some(foreach);
    }

    unsafe fn as_ref(&self) -> &mut StreamInner<T> {
        &mut *(self.0).0
    }
}

impl<T: Send> Stream<T> {
    pub fn new() -> (Stream<T>, StreamSender<T>) {
        StreamInner::new()
    }

    pub fn foreach<F>(self, f: F) where F: FnMut(T) + Send {
        unsafe{self.inner.set_foreach(f)}
    }

    pub fn map<F, V>(self, mut f: F) -> Stream<V> where F: FnMut(T)->V + Send, V: Send {
        let (stream, mut sender) = StreamInner::new();
        let foreach = move |value| sender.send(f(value));
        self.foreach(foreach);
        stream
    }
}

impl<T: Send> StreamSender<T> {
    pub fn send(&mut self, value: T) {
        unsafe{self.inner.send(value)}
    }

    pub fn close(self) {
        unsafe{self.inner.close()}
    }
}




struct MutThunk<A=(),R=()> {
    invoke: Box<Invoke<A,R>+Send>
}

impl<R> MutThunk<(),R> {
    fn new<F>(mut func: F) -> MutThunk<(),R>
        where F : FnMut() -> R, F : Send
    {
        MutThunk::with_arg(move |()| func())
    }
}

impl<A,R> MutThunk<A,R> {
    fn with_arg<F>(func: F) -> MutThunk<A,R>
        where F : FnMut(A) -> R, F : Send
    {
        MutThunk {
            invoke: Box::new(func)
        }
    }

    fn invoke(&mut self, arg: A) -> R {
        self.invoke.invoke(arg)
    }
}

trait Invoke<A=(),R=()> {
    fn invoke(&mut self, arg: A) -> R;
}

impl<A,R,F> Invoke<A,R> for F
    where F : FnMut(A) -> R
{
    fn invoke(&mut self, arg: A) -> R {
        self(arg)
    }
}
