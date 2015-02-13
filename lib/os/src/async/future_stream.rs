use std::mem;
use async::stream::{Stream, StreamSender, Subscriber};
use async::future::{Future, FutureExt};
use async::future_value::{self, FutureValue, FutureValueSetter};

pub struct FutureStream<T> {
    subscriber_setter: FutureValueSetter<Box<Subscriber<Item=T>>>,
}

pub struct FutureStreamSender<T> {
    subscriber: FutureValue<Box<Subscriber<Item=T>>>,
}

impl<T> FutureStream<T> where T: Send {
    pub fn new() -> (FutureStream<T>, FutureStreamSender<T>) {
        let (future, setter) = future_value::new_pair();
        let stream = FutureStream {
            subscriber_setter: setter,
        };
        let sender = FutureStreamSender {
            subscriber: future,
        };
        (stream, sender)
    }
}

impl<T> Stream for FutureStream<T> where T: Send {
    type Item = T;

    fn subscribe<S>(self, subscriber: S) where S: Subscriber<Item=T> {
        self.subscriber_setter.set(Box::new(subscriber));
    }
}

impl<T> StreamSender for FutureStreamSender<T> where T: Send {
    type Item = T;

    fn send(&mut self, value: T) {
        // avoid `cannot move out of borrowed context`
        let mut tmp = unsafe{mem::uninitialized()};
        mem::swap(&mut self.subscriber, &mut tmp);

        tmp = tmp.then_map(|mut subscriber| {
            subscriber.on_value(value);
            subscriber
            });

        mem::swap(&mut self.subscriber, &mut tmp);
        unsafe{mem::forget(tmp)};
    }

    fn close(self) {
        drop(self)
    }
}

#[unsafe_destructor]
impl<T> Drop for FutureStreamSender<T> {
    fn drop(&mut self) {
        let dummy = Box::new(Dummy);
        let s = mem::replace(&mut self.subscriber, FutureValue::from_value(dummy));
        s.then(|subscriber| subscriber.on_close())
    }
}
struct Dummy<T>;
impl<T> Subscriber for Dummy<T> {
    type Item = T;
}