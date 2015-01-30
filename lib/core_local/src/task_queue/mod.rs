pub use self::thunk::Thunk;

use core::atomic::{AtomicBool, Ordering};
use core::marker::Send;
use core::ops::{FnOnce, Deref, DerefMut};
use core::option::Option::{self, Some, None};
use core::result::Result::{self, Ok, Err};
use collections::RingBuf;

mod thunk;

struct TaskQueue {
    locked: AtomicBool,
    tasks: RingBuf<Thunk<(),()>>,
}

pub fn add<F>(f: F) -> Result<(), F> where F: FnOnce(), F: Send {
    match TaskQueue::core_local() {
        Some(mut queue) => {
            queue.push_back(f);
            Ok(())
        },
        None => Err(f),
    }
}

pub fn next() -> Option<Thunk<(),()>> {
    match TaskQueue::core_local() {
        Some(mut queue) => queue.pop_front(),
        None => None,
    }
}

impl TaskQueue {
    pub fn push_back<F>(&mut self, f: F) where F: FnOnce(), F: Send {
        self.tasks.push_back(Thunk::new(f))
    }

    pub fn pop_front(&mut self) -> Option<Thunk<(),()>> {
        self.tasks.pop_front()
    }

    fn core_local() -> Option<TaskQueueRef<'static>> {

        #[cfg(target_arch = "x86_64")]
        fn get_task_queue_ptr() -> *mut TaskQueue {
            let mut data: *mut TaskQueue;
            unsafe{asm!("movq %gs:0, $0" : "=r"(data) ::: "volatile")};
            data
        }

        let ptr = get_task_queue_ptr();
        if unsafe{&*ptr}.locked.compare_and_swap(false, true, Ordering::SeqCst) != false {
            // task queue is locked...
            None
        } else {
            Some(TaskQueueRef(unsafe{&mut *ptr}))
        }
    }
}

struct TaskQueueRef<'a>(&'a mut TaskQueue);

impl<'a> Deref for TaskQueueRef<'a> {
    type Target = TaskQueue;
    fn deref(&self) -> &TaskQueue {
        self.0
    }
}

impl<'a> DerefMut for TaskQueueRef<'a> {
    fn deref_mut(&mut self) -> &mut TaskQueue {
        self.0
    }
}