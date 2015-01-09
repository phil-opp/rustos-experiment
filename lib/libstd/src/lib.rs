// Don't link to std. We are std.
#![no_std]

#[macro_use]
#[macro_reexport(write, writeln)]
extern crate core;

#[macro_use]
#[macro_reexport(vec)]
extern crate "collections" as core_collections;

extern crate "rand" as core_rand;
extern crate alloc;
extern crate unicode;

// NB: These reexports are in the order they should be listed in rustdoc

pub use core::any;
pub use core::borrow;
pub use core::cell;
pub use core::clone;
#[cfg(not(test))] pub use core::cmp;
pub use core::default;
pub use core::finally;
pub use core::hash;
pub use core::intrinsics;
pub use core::iter;
#[cfg(not(test))] pub use core::marker;
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
#[stable]
pub use core_collections::vec;

pub use unicode::char;

/* Exported macros */

#[macro_use]
mod macros;

#[macro_use]
pub mod bitflags;

// A curious inner-module that's not exported that contains the binding
// 'std' so that macro-expanded references to std::error and such
// can be resolved within libstd.
#[doc(hidden)]
mod std {
    // mods used for deriving
    pub use clone;
    pub use cmp;
    pub use hash;
    pub use default;

    pub use option; // used for bitflags!{}
    pub use vec; // used for vec![]
    pub use cell; // used for tls!
    pub use marker;  // used for tls!
    pub use ops; // used for bitflags!

    pub use slice;

    pub use boxed; // used for vec![]

}
