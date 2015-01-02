#![no_std]
#![feature(globs, phase, asm, macro_rules, lang_items, default_type_params, unboxed_closures)]

#[cfg(test)] #[phase(plugin, link)] extern crate log;

#[cfg(not(test))] extern crate "os_alloc" as alloc;
#[cfg(test)] extern crate alloc;
extern crate unicode;
#[phase(plugin, link)] extern crate core;
#[cfg(not(test))] extern crate "os_collections" as core_collections;
#[cfg(test)] extern crate "collections" as core_collections;
extern crate "rand" as core_rand;
extern crate rlibc;
extern crate spinlock;

// Make std testable by not duplicating lang items. See #2912
#[cfg(test)] extern crate "std" as realstd;
#[cfg(test)] pub use realstd::kinds;
#[cfg(test)] pub use realstd::ops;
#[cfg(test)] pub use realstd::cmp;
#[cfg(test)] pub use realstd::boxed;


// NB: These reexports are in the order they should be listed in rustdoc

pub use core::any;
pub use core::borrow;
pub use core::cell;
pub use core::clone;
#[cfg(not(test))] pub use core::cmp;
pub use core::default;
pub use core::finally;
pub use core::intrinsics;
pub use core::iter;
#[cfg(not(test))] pub use core::kinds;
pub use core::mem;
#[cfg(not(test))] pub use core::ops;
pub use core::ptr;
pub use core::raw;
pub use core::simd;
pub use core::result;
pub use core::option;

#[cfg(not(test))] pub use alloc::boxed;
pub use alloc::rc;

pub use core_collections::slice;
pub use core_collections::str;
pub use core_collections::string;
pub use core_collections::vec;

pub use unicode::char;

use core::prelude::*;

/* Exported macros */

pub mod macros;

/* new for os */
mod multiboot;
mod init;
mod vga_buffer;
mod fn_box;
mod scheduler;
mod global;


pub mod fmt;

/* Common data structures */

pub mod collections;


// Documentation for primitive types

mod bool;
mod unit;
mod tuple;


#[no_mangle]
pub fn main(multiboot: *const multiboot::Information) {

    unsafe{init::frame_stack(multiboot)};
    global::init();
    unsafe{enable_interrupts()};

    print!("test\n\niuaeiae");
    let x = box 5i;

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
    loop{
        let mut x = 0u;
            for i in range(0,100000) {
                x = i;
            }
        print!("m");
    }    


    loop{}
    panic!("end of os!");
}

// A curious inner-module that's not exported that contains the binding
// 'std' so that macro-expanded references to std::error and such
// can be resolved within libstd.
#[doc(hidden)]
mod std {
    // mods used for deriving
    pub use clone;
    pub use cmp;
    //pub use hash;

    //pub use comm; // used for select!()
    //pub use error; // used for try!()
    pub use fmt; // used for any formatting strings
    //pub use io; // used for println!()
    pub use option; // used for bitflags!{}
    //pub use rt; // used for panic!()
    pub use vec; // used for vec![]
    pub use cell; // used for tls!
    //pub use thread_local; // used for thread_local!
    pub use kinds; // used for tls!

    // The test runner calls ::std::os::args() but really wants realstd
    #[cfg(test)] pub use realstd::os as os;
    // The test runner requires std::slice::Vector, so re-export std::slice just for it.
    //
    // It is also used in vec![]
    pub use slice;

    pub use boxed; // used for vec![]

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

#[cfg(not(test))]
#[lang = "panic_fmt"] 
fn panic_fmt() -> ! { unimplemented!();}

#[no_mangle]
#[allow(non_snake_case)]
pub extern "C" fn _Unwind_Resume() -> ! {
    unimplemented!();
}
