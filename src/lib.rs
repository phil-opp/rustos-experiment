#![feature(asm, lang_items, unboxed_closures)]

extern crate spinlock;
extern crate scheduler;

mod multiboot;
mod init;

#[no_mangle]
pub fn main(multiboot: *const multiboot::Information) {

    unsafe{
        init::frame_stack(multiboot);
        scheduler::init();
        enable_interrupts();
    };

    print!("test\n\niuaeiae");
    let x = Box::new(5i);

    let y = 0xb8000 as *mut u64;
    unsafe{*y = 0xffffffffffffffff};

    print!("test");
    print!("test\n");
    println!("newline {}", x);


    scheduler::spawn(|| print!("I'm #1!\n"));

    fn test(name: &str) {
        loop {
            let mut x = 0u;
            for i in range(0,100000) {
                x = i;
            }
            print!("{}", name);
        }
    }

    scheduler::spawn(|| test("1"));
    scheduler::spawn(|| test("2"));
    scheduler::spawn(|| test("3"));
    scheduler::spawn(|| test("4"));
    scheduler::spawn(|| test("5"));
    scheduler::spawn(|| test("6"));
    
    test("m");


    loop{}
    panic!("end of os!");
}


/* Interrupt Handlers */

extern {
//    fn send_eoi(interrupt_number: u64) -> int; //return value to mark rax as used
}

unsafe fn out_byte(port: u16, data: u8) {
    asm!("outb %al, %dx" :: "{dx}"(port), "{al}"(data) :: "volatile");
}
unsafe fn in_byte(port: u16) -> u8 {
    let ret: u8;
    asm!("inb %dx, %al" : "={al}"(ret) : "{dx}"(port) :: "volatile");
    ret
}
unsafe fn enable_interrupts() {
    asm!("sti" :::: "volatile");
}
unsafe fn disable_interrupts() {
    asm!("cli" :::: "volatile");
}

unsafe fn send_eoi(interrupt_number: u64) {
    unsafe fn send_master_eoi() {out_byte(0x20, 0x20)}
    unsafe fn send_slave_eoi() {out_byte(0xA0, 0x20)}

    match interrupt_number {
        i if i >= 40 => {
            send_slave_eoi(); 
            send_master_eoi();
        },
        32...40 => send_master_eoi(),
        _ => {},
    }
}

#[no_mangle]
pub extern "C" fn interrupt_handler(interrupt_number: u64, error_code: u64, rsp:uint) {
    match interrupt_number {
        13 if error_code != 0 => panic!(
            "General Protection Fault: Segment error at segment 0x{:x}", error_code),
        32 => {},
        33 => print!("k"),
        50 => panic!("out of memory"),
        66 => println!("ending thread..."),
        _ => panic!("unknown interrupt! number: {}, error_code: {:x}",interrupt_number, error_code),
    };
    unsafe{send_eoi(interrupt_number)};

    //TODO enable interrupts

    //TODO move interrupt numbers to own crate (yield() etc)

    match interrupt_number {
        32 => unsafe{scheduler::reschedule(rsp)},
        66 => unsafe{scheduler::schedule()},
        _ => {},
    }    
}

#[no_mangle]
pub extern "C" fn pagefault_handler(address: u64, error_code: u64, rsp:uint) {
    panic!("page fault: address: {:x}, error_code: {:b}, rsp: {:x}", address, error_code, rsp);
}

#[cfg(not(test))]
#[lang = "stack_exhausted"] 
extern fn stack_exhausted() {panic!("stack exhausted");}

#[cfg(not(test))]
#[lang = "eh_personality"] 
extern fn eh_personality() {unimplemented!();}

#[no_mangle]
#[allow(non_snake_case)]
pub extern "C" fn _Unwind_Resume() -> ! {
    unimplemented!();
}
