use core::prelude::*;
use core::default::Default;
use boxed::Box;
use fn_box::FnBox;
use collections::RingBuf;
use global::global;
use alloc::heap::allocate;
use core::intrinsics::size_of;

pub fn spawn<F, R>(f:F) -> Future<R> where F: FnOnce()->R, F:Send {
    let ret = global().scheduler.lock().spawn(f);

    let stack_bottom = unsafe{allocate(4096*2, 4096) as uint};
    let stack_top = stack_bottom + 4096*2 - unsafe{size_of::<ThreadRegisters>()};
    let registers = stack_top as *mut ThreadRegisters;

    unsafe{
        (*registers).IP = invoke_next_new_thread as u64;
        (*registers).CS = 8;
        (*registers).flags = 0;
    }

    global().scheduler.lock().park(Thread::from_rsp(stack_top));
    ret
}

/// park current thread
pub unsafe fn park_current(current_rsp: uint) {
    print!("parking\n");
    let current = Thread::from_rsp(current_rsp);
    global().scheduler.lock().park(current);
}

/// kill current thread
pub unsafe fn kill_current(current_rsp: uint) {
    // TODO
}

/// schedule new thread
pub unsafe fn schedule() -> Option<uint> {
    global().scheduler.lock().next_thread().map(|t| t.rsp)
}

fn invoke_next_new_thread() {
    print!("hi :) \n");
    match global().scheduler.lock().next_new_thread() {
        Some(t) => {
            print!("executing new\n");
            t.function.call_once(())
        },
        None => panic!("no new thread available")
    }
    unsafe{::kill_current()}
}

pub type GlobalScheduler = Scheduler;
impl GlobalScheduler {
    pub fn new() -> GlobalScheduler {
        Scheduler::new()
    }
}

pub struct Future<T>;

pub struct Thread {
    rsp: uint,
}

impl Thread {
    pub fn from_rsp(rsp: uint) -> Thread {
        Thread {
            rsp: rsp,
        }
    }
}

struct NewThread {
    function: Box<FnBox() + Send>,
}

impl NewThread {
    fn new<F>(f: F) -> NewThread where F : FnOnce(), F: Send {
        NewThread {
            function: box f,
        }
    }
}

pub struct Scheduler {
    threads: RingBuf<Thread>,
    new_threads: RingBuf<NewThread>,
}

impl Scheduler {
    pub fn new() -> Scheduler {
        Scheduler{
            threads: RingBuf::new(),
            new_threads: RingBuf::new(),
        }
    }

    fn spawn<F, R>(&mut self, f:F) -> Future<R> where F: FnOnce()->R, F: Send {
        
        //let (tx, rx) = channel();

        self.new_threads.push_back(NewThread::new(move |:| {
            /*tx.send_opt(*/ f();
        }));

        Future/*::from_receiver(rx)*/
    }

    fn park(&mut self, thread: Thread) {
        print!("{}\n", self.threads.len());
        self.threads.push_back(thread)
    }

    fn next_thread(&mut self) -> Option<Thread> {
        self.threads.pop_front()
    }

    fn next_new_thread(&mut self) -> Option<NewThread> {
        self.new_threads.pop_front()
    }
}

#[packed]
pub struct U128 {
    lower: u64,
    upper: u64
}

#[packed]
pub struct ThreadRegisters {
    pub error_code: u64,
    pub interrupt_number: u64,
    pub stack_limit: u64,
    pub xmm15: U128,
    pub xmm14: U128,
    pub xmm13: U128,
    pub xmm12: U128,
    pub xmm11: U128,
    pub xmm10: U128,
    pub xmm9: U128,
    pub xmm8: U128,
    pub xmm7: U128,
    pub xmm6: U128,
    pub xmm5: U128,
    pub xmm4: U128,
    pub xmm3: U128,
    pub xmm2: U128,
    pub xmm1: U128,
    pub xmm0: U128,
    pub r15: u64,
    pub r14: u64,
    pub r13: u64,
    pub r12: u64,
    pub r11: u64,
    pub r10: u64,
    pub r9: u64,
    pub r8: u64,
    pub rdi: u64,
    pub rsi: u64,
    pub rbp: u64,
    pub rdx: u64,
    pub rcx: u64,
    pub rbx: u64,
    pub rax: u64,
    pub IP: u64,
    pub CS: u64,
    pub flags: u64,
}