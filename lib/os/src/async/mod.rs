pub use self::future::Future;
pub use self::future_stream::Stream;

pub use core_local::task_queue;

mod future;
mod future_stream;
mod stream;
mod spsc_queue;
mod mpsc_queue;

pub fn run<F, R>(f: F) -> Future<R> where F: FnOnce()->R, F: Send, R: Send {
    Future::from_fn(f)
}