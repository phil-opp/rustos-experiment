use std::intrinsics::{size_of, offset};
use std::iter::{Iterator, range_step};

extern {
    static kernel_end_symbol_table_entry: ();
    //static kernel_start_symbol_table_entry: ();
}

struct PhysicalAddress(u64);

struct FrameStack {
    first: *const PhysicalAddress,
    length: u32, //up to 16TB memory possible
}

const PAGE_SIZE: u64 = 4096;
const FRAME_STACK : *const FrameStack = (10*1024*1024 as uint) as *const FrameStack; //10MB

pub unsafe fn frame_stack(multiboot: *const ::multiboot::Information) {

    let kernel_end = (&kernel_end_symbol_table_entry as *const ()) as *const FrameStack;
    assert!(kernel_end < FRAME_STACK);

    let mut areas = (*multiboot).memory_areas().unwrap();    
    let maximal_phys_addr = areas.clone().fold(0, |max, area| {
                if area.base_addr + area.length > max {
                    area.base_addr + area.length
                } else {max}
            });
    let free_phys_addr_start = (FRAME_STACK as u64) + size_of::<FrameStack>() as u64
         + maximal_phys_addr / PAGE_SIZE * size_of::<PhysicalAddress>() as u64;

    let mut stack = FrameStack {
        first: offset(FRAME_STACK, 1) as *const PhysicalAddress, 
        length: 0,
    };

    for area in areas {
        for frame_addr in range_step(area.base_addr, area.base_addr + area.length, PAGE_SIZE) {
            if frame_addr > free_phys_addr_start {
                stack.push(PhysicalAddress(frame_addr))
            }
        }
    }
    *(FRAME_STACK as *mut FrameStack) = stack;
}

impl FrameStack {
    unsafe fn push(&mut self, frame: PhysicalAddress) {
        let last = offset(self.first, self.length as int);
        *(last as *mut PhysicalAddress) = frame;
        self.length += 1;
    }

}