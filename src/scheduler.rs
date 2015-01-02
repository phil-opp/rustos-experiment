use core::prelude::*;
use core::default::Default;
use boxed::Box;
use fn_box::FnBox;
use collections::RingBuf;
use global::global;
use alloc::heap::allocate;
use spinlock::{Spinlock, SpinlockGuard};

pub fn spawn<F, R>(f:F) -> Future<R> where F: FnOnce()->R, F:Send {
    global().scheduler.spawn(f)
}

pub unsafe fn reschedule(current_rsp: uint) -> ! {
    // switch stack so we can park current thread

    call_on_stack(inner, current_rsp, scheduler_stack_top());

    fn inner(current_rsp: uint) -> ! {
        // park current thread
        let current = Thread::from_rsp(current_rsp);
        global().scheduler.park(current);

        schedule_inner();
    }    
}

pub unsafe fn schedule() -> ! {
    call_on_stack(inner, scheduler_stack_top());
    
    unsafe fn call_on_stack(function: fn() -> !, stack_top: uint) -> ! {
        asm!("call $0;" :: "r"(function), "{rsp}"(stack_top) :: "intel", "volatile");
        panic!("diverging fn returned");
    }

    fn inner() -> ! {
        // TODO FIXME kill current thread and deallocate stack
        schedule_inner()
    }
}

fn schedule_inner() -> ! {
    #[allow(improper_ctypes)]
    extern {
        fn pop_registers_and_iret(rsp: uint) -> !;
    }

    fn new_stack() -> uint {
        const NEW_STACK_SIZE: uint = 4096*2;
        let stack_bottom = unsafe{allocate(NEW_STACK_SIZE, 4096)};
        let stack_top = stack_bottom as uint + NEW_STACK_SIZE;
        stack_top
    }

    fn invoke(function: Box<FnBox() + Send>) -> ! {
        unsafe{::enable_interrupts()};
        function.call_once(());
        unsafe{asm!("int $$66" :::: "volatile")};
        unreachable!();
    }

    // schedule new thread
    let new_thread = unsafe{global().scheduler.schedule().unwrap_or_default()};

    match new_thread {
        Thread{state: ThreadState::Active{rsp}} => {
            unsafe{pop_registers_and_iret(rsp)}
        },
        Thread{state: ThreadState::New{function}} => {
            println!("new");
            let new_stack_top = new_stack();
            unsafe{call_on_stack(invoke, function, new_stack_top)}
        },
    }
}

fn scheduler_stack_top() -> uint {
    const SCHEDULER_STACK_SIZE: uint = 4096;
    static SCHEDULER_STACK: [u8; SCHEDULER_STACK_SIZE] = [0; SCHEDULER_STACK_SIZE];

    &SCHEDULER_STACK as *const [u8; 4096] as uint + SCHEDULER_STACK_SIZE
}

unsafe fn call_on_stack<Arg>(function: fn(Arg) -> !, arg: Arg, stack_top: uint) -> ! {
    asm!("call $0;" :: "r"(function), "{rdi}"(arg), "{rsp}"(stack_top) :
        : "intel", "volatile");
    panic!("diverging fn returned");
}

pub struct Future<T>;

pub struct Thread {
    state: ThreadState,
}

impl Thread {
    fn new<F>(f: F) -> Thread where F : FnOnce(), F: Send {
        Thread {
            state: ThreadState::New {
                function: box f,
            }
        }
    }

    pub fn from_rsp(rsp: uint) -> Thread {
        Thread {
            state: ThreadState::Active {
                rsp: rsp,
            }
        }
    }
}

impl Default for Thread {
    fn default() -> Thread {
        Thread::new(|| print!("."))
    }
}

enum ThreadState {
    New {
        function: Box<FnBox() + Send>,
    },
    Active {
        rsp: uint,
    }
}

struct NonInterruptableSpinlock<T>(Spinlock<T>);

impl<T> NonInterruptableSpinlock<T> {
    pub fn lock(&self) -> SpinlockGuard<T> {
        unsafe{::disable_interrupts()};
        self.0.lock()
    }
}

pub struct GlobalScheduler {
    threads: NonInterruptableSpinlock<RingBuf<Thread>>,
}

impl GlobalScheduler {
    pub fn new() -> GlobalScheduler {
        GlobalScheduler{
            threads: NonInterruptableSpinlock(Spinlock::new(RingBuf::new())),
        }
    }

    fn spawn<F, R>(&self, f:F) -> Future<R> where F: FnOnce()->R, F: Send {
        
        //let (tx, rx) = channel();

        self.threads.lock().push_back(Thread::new(move |:| {
            /*tx.send_opt(*/ f();
        }));

        unsafe{::enable_interrupts()};

        Future/*::from_receiver(rx)*/
    }

    fn park(&self, thread: Thread) {
        self.threads.lock().push_back(thread)
    }

    unsafe fn schedule(&self) -> Option<Thread> {
        self.threads.lock().pop_front()
    }
}