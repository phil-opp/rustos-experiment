#![no_std]
#![feature(globs, phase, asm, macro_rules, lang_items)]

#[phase(plugin, link)] extern crate core;
extern crate rlibc;
extern crate "os_alloc" as alloc;
extern crate unicode;
extern crate "os_collections" as core_collections;
extern crate "rand" as core_rand;

// NB: These reexports are in the order they should be listed in rustdoc

pub use core::any;
pub use core::bool;
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
pub use core::tuple;
// FIXME #15320: primitive documentation needs top-level modules, this
// should be `std::tuple::unit`.
pub use core::unit;
pub use core::result;
pub use core::option;

#[cfg(not(test))] pub use alloc::boxed;
pub use alloc::rc;

pub use core_collections::slice;
pub use core_collections::str;
pub use core_collections::string;
pub use core_collections::vec;

pub use unicode::char;

/* Exported macros */

pub mod macros;
pub mod bitflags;

pub mod fmt;

mod multiboot;
mod init;
mod vga_buffer;


#[no_mangle]
pub fn main(multiboot: *const multiboot::Information) {

    unsafe{init::frame_stack(multiboot)};

    unsafe{asm!("sti")};
    print!("test\n\niuaeiae");
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

#[lang = "stack_exhausted"] extern fn stack_exhausted() {unimplemented!();}
#[lang = "eh_personality"] extern fn eh_personality() {unimplemented!();}
#[lang = "panic_fmt"] fn panic_fmt() -> ! { unimplemented!();}

#[no_mangle]
#[allow(non_snake_case)]
pub extern "C" fn _Unwind_Resume() -> ! {
    unimplemented!();
}
