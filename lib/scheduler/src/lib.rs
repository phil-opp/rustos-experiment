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

pub unsafe fn reschedule(current_rsp: uint) -> ! {
    if !current_thread_parkable() {
        pop_registers_and_iret(current_rsp)
    } else {
        inner(current_rsp)
    }

    /* OLD */
    // we must not be interrupted while using the scheduler stack
    // TODO ::disable_interrupts();
    // switch stack so we can park current thread
    //call_on_stack(inner, current_rsp, scheduler_stack_top());

    fn inner(current_rsp: uint) -> ! {

        let current_thread = &mut thread_local::data().borrow_mut().current_thread;
        let scheduler = &global().scheduler;

        current_thread.state = ThreadState::Active{rsp: current_rsp};

        let current = mem::replace(current_thread, scheduler.schedule());
        scheduler.park(current);

        start_current_thread()
    }    
}

pub unsafe fn schedule() -> ! {
    assert!(current_thread_parkable());
    inner();
    /* OLD */
    // we must not be interrupted while using the scheduler stack
    // TODO ::disable_interrupts();
    // switch to scheduler stack (the stack of current thread could be to full/small)
    //call_on_stack(inner, scheduler_stack_top());
    
    /*
    unsafe fn call_on_stack(function: fn() -> !, stack_top: uint) -> ! {
        asm!("call $0;" :: "r"(function), "{rsp}"(stack_top) :: "intel", "volatile");
        panic!("diverging fn returned");
    }
    */

    fn inner() -> ! {
        let old = mem::replace(&mut thread_local::data().borrow_mut().current_thread, 
            global().scheduler.schedule());
        // TODO FIXME kill old thread and deallocate stack
        start_current_thread()
    }
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

    fn invoke(function: Box<FnBox() + Send>) -> ! {
        // TODO unsafe{::enable_interrupts()};
        function.call_once(());
        unsafe{asm!("int $$66" :::: "volatile")};
        unreachable!();
    }
  let current_state = mem::replace(&mut thread_local::data().borrow_mut().current_thread.state,
        ThreadState::Running);
    match current_state {
        ThreadState::Active{rsp} => {
            unsafe{pop_registers_and_iret(rsp)}
        },
        ThreadState::New{function} => {
            println!("new");
            let new_stack_top = new_stack();
            unsafe{call_on_stack(invoke, function, new_stack_top)}
        },
        ThreadState::Running => panic!("current thread must not be running"),
    }
}

/*
fn scheduler_stack_top() -> uint {
    const SCHEDULER_STACK_SIZE: uint = 4096;
    static SCHEDULER_STACK: [u8; SCHEDULER_STACK_SIZE] = [0; SCHEDULER_STACK_SIZE];

    &SCHEDULER_STACK as *const [u8; 4096] as uint + SCHEDULER_STACK_SIZE
}
*/

#[inline(never)]
unsafe fn call_on_stack<Arg>(function: fn(Arg) -> !, arg: Arg, stack_top: uint) -> ! {
    asm!("call $0;" :: "r"(function), "{rdi}"(arg), "{rsp}"(stack_top) :
        : "intel", "volatile");
    panic!("diverging fn returned");
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

    fn schedule(&self) -> Thread {
        self.threads.lock().pop_front().unwrap_or_default()
    }

    fn locked_by_current_thread(&self) -> bool {
        self.threads.held_by_current_thread()
    }
}