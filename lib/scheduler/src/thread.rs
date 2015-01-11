use std::sync::atomic::{AtomicUint, ATOMIC_UINT_INIT, Ordering};
use std::default::Default;
use std::mem;
use fn_box::FnBox;

pub struct Thread {
    pub id: ThreadId,
    pub state: ThreadState,
}

pub struct ThreadId(usize);

impl ThreadId {
	pub fn as_usize(&self) -> usize {
		self.0
	}
}

pub enum ThreadState {
    New {
        function: Box<FnBox() + Send>,
    },
    Active {
        rsp: uint,
    },
    Running,
}

impl Thread {

    fn next_id() -> ThreadId {
    	static NEXT_ID: AtomicUint = ATOMIC_UINT_INIT;
    	ThreadId(NEXT_ID.fetch_add(1, Ordering::Relaxed) + 1) // start at id 1
    }
    
    pub fn new<F>(f: F) -> Thread where F : FnOnce(), F: Send {
        Thread {
        	id: Thread::next_id(),
            state: ThreadState::New {
                function: Box::new(f),
            }
        }
    }

    pub unsafe fn swap_state(&mut self, new: ThreadState) -> ThreadState {
    	let mut old = new;
    	mem::swap(&mut old, &mut self.state);
    	old
    }
}

impl Default for Thread {
    fn default() -> Thread {
        Thread::new(|| print!("."))
    }
}
