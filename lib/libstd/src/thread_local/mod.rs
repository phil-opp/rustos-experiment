use prelude::v1::*;
use cell::UnsafeCell;

macro_rules! thread_local {
    (static $name:ident: $t:ty = $init:expr) => (
        #[thread_local]
        static $name: ::std::thread_local::Key<$t> = {
            use std::option::Option::None;
            use std::cell::UnsafeCell;

            fn init() -> $t {
                $init
            }

            ::std::thread_local::Key {
                value: UnsafeCell{value: None},
                init: init,
            }
        };
    );
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
            f(match *self.value.get() {
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