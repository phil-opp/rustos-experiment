use std::ops::{Deref, DerefMut};
use std::cell::RefCell;
use thread::Thread;

pub struct Data {
    pub current_thread: Thread,
    pub parkable: bool, // i.e. for frame stack lock
}

pub fn data() -> &'static RefCell<Data> {

    #[cfg(target_arch = "x86_64")]
    unsafe fn get_data_ptr() -> *const RefCell<Data> {
        let mut data: *const RefCell<Data>;
        asm!("movq %fs, $0" : "=r"(data) ::: "volatile");
        data
    }
    
    unsafe{&*get_data_ptr()}
}