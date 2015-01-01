
mod frame_stack;

pub unsafe fn frame_stack(multiboot: *const ::multiboot::Information) {
    frame_stack::init_frame_stack(multiboot)
}