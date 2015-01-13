#![feature(asm, unboxed_closures, unsafe_destructor)]

use std::collections::RingBuf;
use std::default::Default;
use std::mem;
use std::rt::heap::allocate;
use global::global;
use spinlock::{Spinlock, SpinlockGuard};
use thread::{Thread, ThreadState};
use fn_box::FnBox;

mod thread;
mod fn_box;
mod global;
mod thread_local;
mod spinlock;

pub fn spawn<F, R>(f:F) -> Future<R> where F: FnOnce()->R, F:Send {
    global().scheduler.spawn(f)
}

/// Can we park the current thread or does it hold an important lock or borrows thread local data
fn current_thread_parkable() -> bool {
    global().scheduler.locked_by_current_thread() || 
    thread_local::data().try_borrow_mut().map_or(false, |d| d.parkable)
}

pub unsafe fn init() {
    global::init();
    thread_local::init(thread_local::Data{
        current_thread: Thread{
            id: mem::transmute(1us),
            state: ThreadState::Running,
        },
        parkable: true,
    })
}

pub unsafe fn reschedule(current_rsp: uint) {
    if current_thread_parkable() {
        if let Some(new_thread) = global().scheduler.schedule() {

            let mut old_thread = replace_current_thread(new_thread);
            old_thread.state = ThreadState::Active{rsp: current_rsp};
            global().scheduler.park(old_thread);

            start_current_thread()    
        }
    }
}

pub unsafe fn schedule() -> ! {
    if current_thread_parkable() {
        let new_thread = if let Some(thread) = global().scheduler.schedule() {
            thread
        } else {
            Default::default()
        };
        let old_thread = replace_current_thread(new_thread);
        // TODO FIXME kill old thread and deallocate stack
        start_current_thread()
    }
    panic!();
}

/// replaces the current thread with a new thread from scheduler and returns the old thread
fn replace_current_thread(new_thread: Thread) -> Thread {
    //println!("scheduling {:?}", new_thread.id);
    let current_thread = &mut thread_local::data().borrow_mut().current_thread;
    mem::replace(current_thread, new_thread)
}

#[allow(improper_ctypes)]
extern {
    fn pop_registers_and_iret(rsp: uint) -> !;
}

fn start_current_thread() -> ! {
    fn new_stack() -> uint {
        const NEW_STACK_SIZE: uint = 4096*2;
        let stack_bottom = unsafe{allocate(NEW_STACK_SIZE, 4096)};
        let stack_top = stack_bottom as uint + NEW_STACK_SIZE;
        stack_top
    }

    #[inline(never)]
    unsafe fn call_on_stack<Arg>(function: fn(Arg) -> !, arg: Arg, stack_top: uint) -> ! {
        asm!("call $0;" :: "r"(function), "{rdi}"(arg), "{rsp}"(stack_top) :
            : "intel", "volatile");
        panic!("diverging fn returned");
    }

    fn invoke(function: Box<FnBox() + Send>) -> ! {
        // TODO unsafe{::enable_interrupts()};
        function.call_once(());
        unsafe{asm!("int $$66" :::: "volatile")};
        unreachable!();
    }

    unsafe fn enable_interrupts() {
        asm!("sti" :::: "volatile");
    }

    let current_state = mem::replace(&mut thread_local::data().borrow_mut().current_thread.state,
        ThreadState::Running);

    unsafe{enable_interrupts()};

    match current_state {
        ThreadState::Active{rsp} => {
            unsafe{pop_registers_and_iret(rsp)}
        },
        ThreadState::New{function} => {
            //println!("new");
            let new_stack_top = new_stack();
            unsafe{call_on_stack(invoke, function, new_stack_top)}
        },
        ThreadState::Running => panic!("current thread must not be running"),
    }
}

pub struct Future<T>;

pub struct GlobalScheduler {
    threads: Spinlock<RingBuf<Thread>>,
}

impl GlobalScheduler {
    pub fn new() -> GlobalScheduler {
        GlobalScheduler{
            threads: Spinlock::new(RingBuf::new()),
        }
    }

    fn spawn<F, R>(&self, f:F) -> Future<R> where F: FnOnce()->R, F: Send {
        
        //let (tx, rx) = channel();

        self.threads.lock().push_back(Thread::new(move |:| {
            /*tx.send_opt(*/ f();
        }));

        Future/*::from_receiver(rx)*/
    }

    fn park(&self, thread: Thread) {
        self.threads.lock().push_back(thread)
    }

    fn schedule(&self) -> Option<Thread> {
        self.threads.lock().pop_front()
    }

    fn locked_by_current_thread(&self) -> bool {
        self.threads.held_by_current_thread()
    }
}