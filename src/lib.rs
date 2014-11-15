#![no_std]
#![feature(globs, phase, asm)]

extern crate rlibc;
extern crate "os_std" as std;
#[phase(plugin)] extern crate "os_std" as std; //macros

pub use std::prelude::*;

extern {
    static kernel_end_symbol_table_entry: ();
    //static kernel_start_symbol_table_entry: ();
}

#[no_mangle]
pub fn main(multiboot: *const std::multiboot::Information) {

    unsafe{std::init(&kernel_end_symbol_table_entry as *const (), multiboot)};
    unsafe{asm!("sti")};
    let x = box 5i;

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
    panic!("page fault: address: {}, error_code: {}, rsp: {}", address, error_code, rsp);
}