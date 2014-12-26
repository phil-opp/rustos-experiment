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

pub unsafe fn schedule(current_rsp: uint) -> uint {
    // park current thread
    let current = Thread::from_rsp(current_rsp);
    global().scheduler.lock().park(current);
    // schedule new thread
    loop {
        match global().scheduler.lock().schedule().unwrap_or_default() {
            Thread{state: ThreadState::Active{rsp}} => return rsp,
            Thread{state: ThreadState::New{function}} => {

                print!("creating thread...");

                let stack_bottom = allocate(4096*2, 4096);
                let stack_top = (stack_bottom as uint + 4096*2);

                print!("bottom {:x} top {:x}\n", stack_bottom as uint, stack_top as uint);
                panic!("done\n");


                asm!("
                    mov rdi, rsp;   // backup stack pointer
                    mov rsp, $2;    // load new stack pointer
                    push rdi;       // push our stack pointer to new stack
                    mov rdi, $1;    // load argument
                    call $0;        // call invocation fn on new stack 
                    pop rsp;        // restore old stack
                    " 
                    :: "r"(invoke), "r"(function), "r"(stack_top) : "rdi" : "intel");
            },
        }
    }
}

extern "C" fn invoke(function: Box<FnBox() + Send>) {
    function.call_once(())
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