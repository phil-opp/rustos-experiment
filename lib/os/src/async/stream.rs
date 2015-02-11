use std::mem;
use async::Future;
use async::computation::{self, Computation, ComputationResultSetter};

pub trait Stream {
    type Item: Send;

    fn subscribe<S>(self, subscriber: S) where S: Subscriber<Item=Self::Item>;
}

pub trait StreamSender {
    type Item: Send;

    fn send(&mut self, value: Self::Item);

    fn close(self);
}

pub trait Subscriber: Send {
    type Item;

    fn on_value(&mut self, value: Self::Item) {}

    fn on_close(self: Box<Self>) {}
}

pub trait StreamExt: Stream + Sized {
    fn map<B, F>(self, f: F) -> Map<Self, F> where 
        F: FnMut(Self::Item) -> B + Send, 
    {
        Map{stream: self, f: f}
    }

    fn fold<B, F>(self, init: B, f: F) -> FoldFuture<Self, B, F> where B: Send, 
        F: FnMut(B, Self::Item) -> B + Send,
    {
        FoldFuture::new(self, init, f)
    }

    fn foreach<F>(self, f: F) where F: FnMut(Self::Item) + Send {
        self.subscribe(ForeachSubscriber{f: f})
    }
}

impl<Strm> StreamExt for Strm where Strm: Stream {}

#[must_use = "stream adaptors are lazy and do nothing unless consumed"]
pub struct Map<Strm, F> {
    stream: Strm,
    f: F,
}

impl<Strm, F, B: Send> Stream for Map<Strm, F> where 
    Strm: Stream,
    F: FnMut(<Strm as Stream>::Item) -> B + Send,
{
    type Item = B;

    fn subscribe<S>(self, subscriber: S) where S: Subscriber<Item=B> {
        let Map{stream, mut f} = self;
        stream.subscribe(MapSubscriber{f: f, subscriber: Box::new(subscriber)})
    }
}

struct MapSubscriber<I, F, S>
{
    f: F,
    subscriber: Box<S>,
}

impl<I, F, S> Subscriber for MapSubscriber<I, F, S> where 
    F: FnMut(I) -> <S as Subscriber>::Item + Send,
    S: Subscriber,
{
    type Item = I;

    fn on_value(&mut self, value: I) {
        self.subscriber.on_value((self.f)(value))
    }

    fn on_close(self: Box<Self>) {
        self.subscriber.on_close()
    }
}

#[must_use = "stream adaptors are lazy and do nothing unless consumed"]
pub struct FoldFuture<Strm, B, F> where B: Send, Strm: Stream {
    stream: Strm,
    future: Computation<B>,
    subscriber: FoldSubscriber<<Strm as Stream>::Item, B, F>
}

impl<Strm, B, F> FoldFuture<Strm, B, F> where Strm: Stream, B: Send, 
    F: FnMut(B, <Strm as Stream>::Item) -> B + Send    
{
    fn new(stream: Strm, init: B, f: F) -> FoldFuture<Strm, B, F> {
        let (future, setter) = computation::new_pair();
        FoldFuture {
            stream: stream,
            future: future,
            subscriber: FoldSubscriber {
                accumulator: init,
                f: f,
                future_setter: setter,
            },
        }
    }
}

impl<Strm, B, F> Future for FoldFuture<Strm, B, F> where Strm: Stream + Send, B: Send, 
    F: FnMut(B, <Strm as Stream>::Item) -> B + Send 
{
    type Item = B;

    fn then<G>(self, g: G) where G: FnOnce(B) + Send {
        let FoldFuture{stream, future, subscriber} = self;
        future.then(g);
        stream.subscribe(subscriber);
    }
}

struct FoldSubscriber<I, B, F> where B: Send {
    accumulator: B,
    f: F,
    future_setter: ComputationResultSetter<B>,
}

impl<I, B, F> Subscriber for FoldSubscriber<I, B, F> where B: Send,
    F: FnMut(B, I) -> B + Send
{
    type Item = I;

    fn on_value(&mut self, value: I) {
        // avoid `cannot move out of borrowed context`
        let mut tmp = unsafe{mem::uninitialized()};
        mem::swap(&mut self.accumulator, &mut tmp);
        tmp = (self.f)(tmp, value);
        mem::swap(&mut self.accumulator, &mut tmp);
        unsafe{mem::forget(tmp)};
    }

    fn on_close(self: Box<Self>) {
        let s = *self;
        s.future_setter.set(s.accumulator)
    }
}

struct ForeachSubscriber<I, F> {
    f: F,
}

impl<I, F> Subscriber for ForeachSubscriber<I, F> where F: FnMut(I) + Send {
    type Item = I;

    fn on_value(&mut self, value: I) {
        (self.f)(value)
    }
}