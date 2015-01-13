macro_rules! panic {
    () => (asm!("int $$50"::::"volatile"));
    ($msg:expr) => ({
        print!($msg);
        panic!()
    });
}

macro_rules! assert(
    ($cond:expr) => (
        if !$cond {
            panic!()
        }
    );
);

macro_rules! print(
    ($($arg:tt)*) => (::vga::print_args(format_args!($($arg)*)))
);