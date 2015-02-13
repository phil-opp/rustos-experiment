pub use self::future::{Future, FutureExt};
pub use self::computation::Computation;
pub use self::stream::{Stream, StreamSender, Subscriber, StreamExt};
pub use self::future_stream::FutureStream;
pub use self::spsc_stream::SpscStream;
//pub use self::spsc_stream::Stream;

pub use core_local::task_queue;

mod future;
mod computation;
mod future_stream;
mod stream;
mod spsc_stream;
mod spsc_queue;
mod mpsc_queue;

pub fn run<F, R>(f: F) -> Computation<R> where F: FnOnce()->R, F: Send, R: Send {
    Computation::new(f)
}