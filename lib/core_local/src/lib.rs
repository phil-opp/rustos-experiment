#![no_std]
#![feature(box_syntax, asm)]

extern crate core;
extern crate collections;
extern crate alloc;

pub use task_queue::{add, next, Thunk};

mod task_queue;
