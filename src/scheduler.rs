use std::collections::RingBuf;
use std::default::Default;
use std::rt::heap::allocate;
use fn_box::FnBox;
use global::global;
use spinlock::{Spinlock, SpinlockGuard};

pub fn spawn<F, R>(f:F) -> Future<R> where F: FnOnce()->R, F:Send {
    global().scheduler.spawn(f)
}

pub unsafe fn reschedule(current_rsp: uint) -> ! {
    // we must not be interrupted while using the scheduler stack
    ::disable_interrupts();
    // switch stack so we can park current thread
    call_on_stack(inner, current_rsp, scheduler_stack_top());

    fn inner(current_rsp: uint) -> ! {
        // try to park current thread
        let current = Thread::from_rsp(current_rsp);
        let scheduler = &global().scheduler;

        let thread = match scheduler.try_park(current) {
            Ok(()) => scheduler.schedule(),
            Err(thread) => {
                println!("thread list is locked. Maybe a thread was interrupted while holding
                    the lock in spawn()?"); 
                thread
            },
        };
        start_thread(thread)
    }    
}

pub unsafe fn schedule() -> ! {
    // we must not be interrupted while using the scheduler stack
    ::disable_interrupts();
    // switch to scheduler stack (the stack of current thread could be to full/small)
    call_on_stack(inner, scheduler_stack_top());
    
    unsafe fn call_on_stack(function: fn() -> !, stack_top: uint) -> ! {
        asm!("call $0;" :: "r"(function), "{rsp}"(stack_top) :: "intel", "volatile");
        panic!("diverging fn returned");
    }

    fn inner() -> ! {
        // TODO FIXME kill current thread and deallocate stack
        start_thread(global().scheduler.schedule())
    }
}

fn start_thread(thread: Thread) -> ! {
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

    match thread {
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
                function: Box::new(f),
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

    fn try_park(&self, thread: Thread) -> Result<(), Thread> {
        match self.threads.try_lock() {
            Some(mut threads) => Ok(threads.push_back(thread)),
            None => Err(thread),
        }
    }

    fn schedule(&self) -> Thread {
        self.threads.lock().pop_front().unwrap_or_default()
    }
}