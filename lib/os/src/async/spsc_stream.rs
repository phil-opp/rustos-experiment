use std::boxed;
use std::sync::Arc;
use std::sync::atomic::{AtomicPtr, AtomicBool, Ordering};
use std::ptr::{self, PtrExt, Unique};
use async::{self, Stream, StreamSender, Subscriber};
use super::spsc_queue::Queue;

pub struct SpscStream<T> {
    inner: Arc<SpscStreamInner<T>>,
}

impl<T: Send> SpscStream<T> {
    pub fn new() -> (SpscStream<T>, SpscStreamSender<T>) {
        let inner = Arc::new(SpscStreamInner{
            queue: unsafe{Queue::new(0)},
            receiver: AtomicPtr::new(ptr::null_mut()),
            closed: AtomicBool::new(false),
        });
        let stream = SpscStream{inner: inner.clone()};
        let sender = SpscStreamSender{inner: inner};
        (stream, sender)
    }
}

impl<T: Send> Stream for SpscStream<T> {
    type Item = T;

    fn subscribe<S>(self, subscriber: S) where S: Subscriber<Item=T> {
        let receiver = Box::new(SpscStreamReceiver {
                inner: self.inner.clone(),
                subscriber: Box::new(subscriber),
                running: AtomicBool::new(false),
        });
        unsafe{self.inner.set_receiver(receiver)}
    }
}

pub struct SpscStreamSender<T> {
    inner: Arc<SpscStreamInner<T>>,
}

impl<T: Send> StreamSender for SpscStreamSender<T> {
    type Item = T;

    fn send(&mut self, value: T) {
        self.inner.queue.push(value);
        self.inner.new_values();
    }

    fn close(self) { /* drop */ }
}

#[unsafe_destructor]
impl<T: Send> Drop for SpscStreamSender<T> {
    fn drop(&mut self) {
        self.inner.closed.store(true, Ordering::SeqCst);
        self.inner.new_values();
    }
}

struct SpscStreamReceiver<T> {
    inner: Arc<SpscStreamInner<T>>,
    running: AtomicBool,
    subscriber: Box<Subscriber<Item=T>>,
}

unsafe impl<T: Send> Sync for SpscStreamReceiver<T> {}

struct SpscStreamInner<T> {
    queue: Queue<T>,
    receiver: AtomicPtr<SpscStreamReceiver<T>>,
    closed: AtomicBool,
}

impl<T: Send> SpscStreamInner<T> {
    unsafe fn set_receiver(&self, receiver: Box<SpscStreamReceiver<T>>) {
        self.receiver.store(boxed::into_raw(receiver), Ordering::SeqCst);
    }

    fn new_values(&self) {
        if let Some(receiver) = unsafe{self.receiver.load(Ordering::SeqCst).as_mut()} {
            // subscriber was set
            if receiver.running.compare_and_swap(false, true, Ordering::SeqCst) == false {
                // start receiver
                async::run(move || {
                    let SpscStreamReceiver{
                        ref inner, ref mut subscriber, ref running,
                    } = *receiver;

                    while let Some(value) = inner.queue.pop() {
                        subscriber.on_value(value);
                    }
                    // mark receiver as not running
                    running.store(false, Ordering::SeqCst);
                    
                    // maybe there was a send between last pop and running.store(false)
                    // -> invoke new_values again
                    if inner.queue.peek().is_some() {
                        inner.new_values();
                    } else if inner.closed.load(Ordering::SeqCst) {
                        println!(" CLOSE ");
                        // take receiver
                        let ptr = inner.receiver.swap(ptr::null_mut(), Ordering::SeqCst);
                        assert!(!ptr.is_null());
                        let receiver = unsafe{Box::from_raw(ptr)};
                        receiver.subscriber.on_close();
                    }
                });
            }
        }        
    }
}