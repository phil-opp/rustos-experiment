#![no_std]
#![feature(globs, phase, asm)]

#[phase(plugin, link)] extern crate "os_std" as std;
pub use std::prelude::*;

mod multiboot;
mod init;


#[no_mangle]
pub fn main(multiboot: *const multiboot::Information) {

    unsafe{init::frame_stack(multiboot)};
    
    unsafe{asm!("sti")};
    let x = box 5i;
    panic!();

    let y = 0xb8000 as *mut u64;
    unsafe{*y = 0xffffffffffffffff};

    print!("test");
    print!("test\n");
    println!("newline {}", x);

    loop {
        panic!("end of os!");
        // Add code here
    }
}


/* Interrupt Handlers */

#[no_mangle]
pub extern "C" fn interrupt_handler(interrupt_number: u64, error_code: u64, rsp:u64) -> u64 {
    match interrupt_number {
        13 if error_code != 0 => panic!(
            "General Protection Fault: Segment error at segment 0x{:x}", error_code),
        32 => {},
        33 => print!("k"),
        _ => panic!("unknown interrupt! number: {}, error_code: {:x}",interrupt_number, error_code),
    };

    rsp
}

#[no_mangle]
pub extern "C" fn pagefault_handler(address: u64, error_code: u64, rsp:u64) -> u64 {
    panic!("page fault: address: {:x}, error_code: {:b}, rsp: {:x}", address, error_code, rsp);
}