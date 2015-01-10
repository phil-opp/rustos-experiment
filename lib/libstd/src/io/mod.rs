
mod vga_buffer;

pub mod stdio {
    use fmt::Arguments;
    use super::vga_buffer;

    pub fn print_args(fmt: Arguments) {
        vga_buffer::print_args(fmt);
    }

    pub fn println_args(fmt: Arguments) {
        vga_buffer::println_args(fmt);
    }

    pub fn print_err_args(fmt: Arguments, file_line: &(&'static str, uint)) {
        vga_buffer::print_err_args(fmt, file_line);
    }
}