use prelude::v1::*;
use cell::{UnsafeCell, RefCell};
use mem;
use core::atomic::{AtomicUsize, ATOMIC_USIZE_INIT, Ordering};

macro_rules! thread_local {
    (static $name:ident: $t:ty = $init:expr) => (
        //#[thread_local]
        static $name: ::std::thread_local::Slot<$t> = {
            use std::option::Option::None;
            use std::cell::UnsafeCell;
            use std::sync::atomic::ATOMIC_USIZE_INIT;

            fn init() -> $t {
                $init
            }

            ::std::thread_local::Slot {
            //::std::thread_local::Key {
                //value: UnsafeCell{value: None},
                number: ATOMIC_USIZE_INIT,
                init: init,
            }
        };
    );
}

pub struct Slot<T> {
    pub number: AtomicUsize,
    pub init: fn() -> T,
}

impl<T: 'static> Slot<T> {

    pub fn with<F, R>(&'static self, f: F) -> R
                      where F: FnOnce(&T) -> R {
        static NEXT_SLOT: AtomicUsize = ATOMIC_USIZE_INIT;
                panic!("1");
        if self.number.load(Ordering::Relaxed) == 0 {
            let number = NEXT_SLOT.fetch_add(1, Ordering::SeqCst) + 16;
            if self.number.compare_and_swap(0, number, Ordering::SeqCst) == 0 {
                let value = (self.init)();
                unsafe{set_data_ptr(Box::new(value), number)};
            }
        }

        unsafe{f(&*get_data_ptr(self.number.load(Ordering::Relaxed)))}
    }
}

#[cfg(target_arch = "x86_64")]
unsafe fn get_data_ptr<T>(slot_number: usize) -> *const T {
    let mut data: *const T;
    asm!("movq %fs:($1), $0" : "=r"(data) : "r"(slot_number) :: "volatile");
    data
}
#[cfg(target_arch = "x86_64")]
unsafe fn set_data_ptr<T>(data: Box<T>, slot_number: usize) {
    let data_ptr: *const T = mem::transmute(data);
    asm!("movq $0, %fs:($1)" :: "r"(data_ptr), "r"(slot_number) :: "volatile");
    assert!(get_data_ptr(slot_number) == data_ptr);
}

pub struct Key<T> {
    pub value: UnsafeCell<Option<T>>,
    pub init: fn() -> T,
} 

unsafe impl<T> Sync for Key<T> {}

impl<T: 'static> Key<T> {
    /// Acquire a reference to the value in this TLS key.
    ///
    /// This will lazily initialize the value if this thread has not referenced
    /// this key yet.
    ///
    /// # Panics
    ///
    /// This function will `panic!()` if the key currently has its
    /// destructor running, and it **may** panic if the destructor has
    /// previously been run for this thread.
    #[stable]
    pub fn with<F, R>(&'static self, f: F) -> R
                      where F: FnOnce(&T) -> R {
        unsafe {
            let test = self.value.get();
            println!("{:x}", test as usize);
            //loop{}
            //f(match *self.value.get() {
            f(match self.value.value {
                Some(ref inner) => inner,
                None => self.init(&self.value),
            })
        }
    }

    unsafe fn init(&self, slot: &UnsafeCell<Option<T>>) -> &T {
        // Execute the initialization up front, *then* move it into our slot,
        // just in case initialization fails.
        let value = (self.init)();
        let ptr = slot.get();
        *ptr = Some(value);
        (*ptr).as_ref().unwrap()
    }
}