use std::mem;
use std::ptr::Unique;
use std::sync::atomic::{AtomicBool, Ordering};
use super::spsc_queue::Queue;
use core_local::task_queue::{self, Thunk};
use async;
use async::future::{Future, FutureSetter};

pub struct Stream<T: Send> {
    foreach_setter: FutureSetter<MutThunk<T>>,
}

pub struct StreamSender<T: Send> {
    foreach: Future<MutThunk<T>>,
}

impl<T: Send> Stream<T> {
    pub fn new() -> (Stream<T>, StreamSender<T>) {
        let (future, setter) = Future::new();
        let stream = Stream {
            foreach_setter: setter,
        };
        let stream_sender = StreamSender {
            foreach: future,
        };
        (stream, stream_sender)
    }

    pub fn foreach<F>(self, f: F) where F: FnMut(T) + Send {
        self.foreach_setter.set(MutThunk::with_arg(f))
    }

    pub fn map<F, V>(self, mut f: F) -> Stream<V> where F: FnMut(T)->V + Send, V: Send {
        let (stream, mut sender) = Stream::new();
        self.foreach(move |value| sender.send(f(value)));
        stream
    }
}

impl<T: Send> StreamSender<T> {
    pub fn send(&mut self, value: T) {
        let mut tmp = unsafe{mem::uninitialized()};
        mem::swap(&mut self.foreach, &mut tmp);

        tmp = tmp.then(|mut f| {
            f.invoke(value);
            f
        });

        tmp = mem::replace(&mut self.foreach, tmp);
        unsafe{mem::forget(tmp)};
    }
}

struct MutThunk<A=(),R=()> {
    invoke: Box<Invoke<A,R>+Send>
}

impl<R> MutThunk<(),R> {
    fn new<F>(mut func: F) -> MutThunk<(),R>
        where F : FnMut() -> R, F : Send
    {
        MutThunk::with_arg(move|&mut: ()| func())
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
