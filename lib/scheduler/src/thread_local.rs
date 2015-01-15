use std::cell::RefCell;
use std::mem;
use thread::Thread;

pub struct Data {
    pub current_thread: Thread,
    pub parkable: bool, // i.e. for frame stack lock
}

pub unsafe fn init(data: Data) {
    set_data_ptr(Box::new(RefCell::new(data)));
}

pub fn data() -> &'static RefCell<Data> {   
    unsafe{&*get_data_ptr()}
}

#[cfg(target_arch = "x86_64")]
unsafe fn get_data_ptr() -> *const RefCell<Data> {
    let mut data: *const RefCell<Data>;
    asm!("movq %fs:0, $0" : "=r"(data) ::: "volatile");
    data
}
#[cfg(target_arch = "x86_64")]
unsafe fn set_data_ptr(data: Box<RefCell<Data>>) {
    let data_ptr: *const RefCell<Data> = mem::transmute(data);
    asm!("movq $0, %fs:0" :: "r"(data_ptr) :: "volatile");
}