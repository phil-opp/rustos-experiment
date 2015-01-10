use prelude::v1::*;

// Reexport some functionality from liballoc.
pub use alloc::heap;

use any::Any;
use fmt;

#[cfg(not(test))]
/// Entry point of panic from the libcore crate.
#[lang = "panic_fmt"]
pub extern fn rust_begin_unwind(msg: fmt::Arguments,
                                file: &'static str, line: uint) -> ! {
    begin_unwind_fmt(msg, &(file, line))
}

/// The entry point for unwinding with a formatted message.
///
/// This is designed to reduce the amount of code required at the call
/// site as much as possible (so that `panic!()` has as low an impact
/// on (e.g.) the inlining of other functions as possible), by moving
/// the actual formatting into this shared place.
#[inline(never)] #[cold]
pub fn begin_unwind_fmt(msg: fmt::Arguments, file_line: &(&'static str, uint)) -> ! {
    loop{}
    //TODO

    //use fmt::Writer;

    // We do two allocations here, unfortunately. But (a) they're
    // required with the current scheme, and (b) we don't handle
    // panic + OOM properly anyway (see comment in begin_unwind
    // below).

    //let mut s = String::new();
    //let _ = write!(&mut s, "{}", msg);
    //begin_unwind_inner(box s, file_line)
}

/// This is the entry point of unwinding for panic!() and assert!().
#[inline(never)] #[cold] // avoid code bloat at the call sites as much as possible
pub fn begin_unwind<M: Any + Send>(msg: M, file_line: &(&'static str, uint)) -> ! {
    // Note that this should be the only allocation performed in this code path.
    // Currently this means that panic!() on OOM will invoke this code path,
    // but then again we're not really ready for panic on OOM anyway. If
    // we do start doing this, then we should propagate this allocation to
    // be performed in the parent of this thread instead of the thread that's
    // panicking.

    // see below for why we do the `Any` coercion here.
    // TODO
    //begin_unwind_inner(box msg, file_line)
    loop{}
}

