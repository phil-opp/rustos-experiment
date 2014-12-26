#![allow(dead_code)]

use core::prelude::*;
use core::mem;
use spinlock::Spinlock;

use scheduler::GlobalScheduler;

pub struct Global {
    pub scheduler: Spinlock<GlobalScheduler>,
}

static mut GLOBAL: *const Global = 0 as *const Global;

pub fn init() {
    unsafe {
        GLOBAL = mem::transmute(box Global{
            scheduler: Spinlock::new(GlobalScheduler::new()),
        });
    };
}

pub fn global() -> &'static Global {
    unsafe{&*GLOBAL}
}