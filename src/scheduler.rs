use core::prelude::*;
use core::default::Default;
use boxed::Box;
use fn_box::FnBox;
use collections::RingBuf;
use global::global;
use alloc::heap::allocate;

pub fn spawn<F, R>(f:F) -> Future<R> where F: FnOnce()->R, F:Send {
    global().scheduler.lock().spawn(f)
}

pub unsafe fn reschedule(current_rsp: uint) -> ! {

    // switch stack so we can park current thread
    const SCHEDULER_STACK_SIZE: uint = 4096;
    static SCHEDULER_STACK: [u8; SCHEDULER_STACK_SIZE] = [0; SCHEDULER_STACK_SIZE];

    let scheduler_stack_top = &SCHEDULER_STACK as *const [u8; 4096] as uint
        + SCHEDULER_STACK_SIZE;
    call_on_stack(inner, current_rsp, scheduler_stack_top);

    fn new_stack() -> uint {
        const NEW_STACK_SIZE: uint = 4096*2;
        let stack_bottom = unsafe{allocate(NEW_STACK_SIZE, 4096)};
        let stack_top = (stack_bottom as uint + NEW_STACK_SIZE);
        stack_top
    }

    fn call_on_stack<Arg>(function: fn(Arg) -> !, arg: Arg, stack_top: uint) -> ! {
        unsafe{asm!("call $0;" :: "r"(function), "{rdi}"(arg), "{rsp}"(stack_top) :
            : "intel", "volatile")}
        panic!("diverging fn returned");
    }

    fn inner(current_rsp: uint) -> ! {
        extern {
            fn pop_registers_and_iret(rsp: uint) -> !;
        }

        // park current thread
        let current = Thread::from_rsp(current_rsp);
        global().scheduler.lock().park(current);

        // schedule new thread
        loop {
            print!("_");
            let new_thread = unsafe{global().scheduler.lock().schedule().unwrap_or_default()};
            match new_thread {
                Thread{state: ThreadState::Active{rsp}} => {
                    unsafe{pop_registers_and_iret(rsp)}
                },
                Thread{state: ThreadState::New{function}} => {
                    println!("new");
                    let new_stack_top = new_stack();
                    let invoke_fn : fn(Box<FnBox() + Send>) -> ! = invoke;
                    call_on_stack(invoke_fn, function, new_stack_top)
                },
            }
        }
    }

    fn invoke(function: Box<FnBox() + Send>) -> ! {
        function.call_once(());
        unsafe{asm!("int $$66" :::: "volatile")};
        unreachable!();
    }
    
}

pub type GlobalScheduler = Scheduler;
impl GlobalScheduler {
    pub fn new() -> GlobalScheduler {
        Scheduler::new()
    }
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


pub struct Scheduler {
    threads: RingBuf<Thread>,
}

impl Scheduler {
    pub fn new() -> Scheduler {
        Scheduler{
            threads: RingBuf::new(),
        }
    }

    fn spawn<F, R>(&mut self, f:F) -> Future<R> where F: FnOnce()->R, F: Send {
        
        //let (tx, rx) = channel();

        self.threads.push_back(Thread::new(move |:| {
            /*tx.send_opt(*/ f();
        }));

        Future/*::from_receiver(rx)*/
    }

    fn park(&mut self, thread: Thread) {
        self.threads.push_back(thread)
    }

    unsafe fn schedule(&mut self) -> Option<Thread> {
        self.threads.pop_front()
    }
}