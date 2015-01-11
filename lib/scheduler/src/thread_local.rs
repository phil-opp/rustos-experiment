use std::ops::{Deref, DerefMut};
use std::cell::{RefCell, Ref, RefMut};
use thread::Thread;

pub struct Data {
    pub current_thread: Thread,
}

pub fn borrow() -> Ref<'static, Data> {

    #[cfg(target_arch = "x86_64")]
    unsafe fn get_data_ptr() -> *const RefCell<Data> {
        let mut data: *const RefCell<Data>;
        asm!("movq %fs, $0" : "=r"(data) ::: "volatile");
        data
    }
    
    unsafe{(*get_data_ptr()).borrow()}
}

pub fn borrow_mut() -> RefMut<'static, Data> {

    #[cfg(target_arch = "x86_64")]
    unsafe fn get_data_ptr() -> *const RefCell<Data> {
        let mut data: *const RefCell<Data>;
        asm!("movq %fs, $0" : "=r"(data) ::: "volatile");
        data
    }
    
    unsafe{(*get_data_ptr()).borrow_mut()}
}
