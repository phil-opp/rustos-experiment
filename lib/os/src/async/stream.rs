use async::Future;

pub trait Stream {
    type Item: Send;

    fn subscribe<S>(self, subscriber: S) where S: Subscriber<Item=Self::Item>;
}

pub trait Subscriber: Send {
    type Item;

    fn on_value(&mut self, value: Self::Item);

    fn on_end(self);
}

pub trait StreamExt: Stream + Sized {
    fn map<B, F>(self, f: F) -> Map<Self, F> where 
        F: FnMut(Self::Item) -> B + Send, 
    {
        Map{stream: self, f: f}
    }

    /*
    fn fold<B, F>(self, init: B, f: F) -> Fold<Self, F> where
        F: FnMut(B, Self::Item) -> B,
    {
        Fold{stream: self, accumulator: init, f: f}
    }*/
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
        
        fn test<T>(s: T) where T: Subscriber {}
        let Map{stream, mut f} = self;
        //test(MapSubscriber{f: f, subscriber: subscriber});
        stream.subscribe(MapSubscriber{f: f, subscriber: subscriber})
    }
}

struct MapSubscriber<I, F, S>
{
    f: F,
    subscriber: S,
}

impl<I, F, S> Subscriber for MapSubscriber<I, F, S> where 
    F: FnMut(I) -> <S as Subscriber>::Item + Send,
    S: Subscriber,
{
    type Item = I;

    fn on_value(&mut self, value: I) {
        self.subscriber.on_value((self.f)(value))
    }

    fn on_end(self) {
        self.subscriber.on_end()
    }
}

/*
#[must_use = "stream adaptors are lazy and do nothing unless consumed"]
pub struct Fold<S, F> {
    stream: S,
    f: F,
}

impl<S: Stream, F, B: Send> Future for Fold<S, B, F> where F: FnMut(B, Self::Item) + Send {
    type Item = B;

    fn then<G>(self, g: G) where G: FnOnce(Self::Item) + Send {
        stream.on_value(move |item| )
    }
}

struct FoldSubscriber<F, S, A> {
    f: F
}
*/