#![allow(dead_code)]

use std::mem;
use std::collections::RingBuf;
use GlobalScheduler;

pub struct Global {
    pub scheduler: GlobalScheduler,
}

static mut GLOBAL: *const Global = 0 as *const Global;

pub fn init() {
    unsafe {
        GLOBAL = mem::transmute(Box::new(Global{
            scheduler: GlobalScheduler::new(),
        }));
        fn require_sync<T>(_: *const T) where T: Sync {}
        require_sync(GLOBAL)
    };
}

pub fn global() -> &'static Global {
    unsafe{&*GLOBAL}
}