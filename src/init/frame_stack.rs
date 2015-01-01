extern crate "os_x86_64_frame_stack" as frame_stack;

use core::prelude::*;
use self::frame_stack::Frame;

extern {
    static kernel_end_symbol_table_entry: ();
    //static kernel_start_symbol_table_entry: ();
}

const PAGE_SIZE: u64 = 4096;

pub unsafe fn init_frame_stack(multiboot: *const ::multiboot::Information) {

    let kernel_end = &kernel_end_symbol_table_entry as *const () as u32;

    let stack_start_frame = Frame{number: kernel_end >> 12};

    // map frame stack to 2mb behind kernel
    map_p1_entries(0, 0, stack_start_frame);
    // map p1 entries
    for i in range(0, 512) {
        map_p1_entries(0, i, Frame{
            number: stack_start_frame.number + i,
        });
    }


    frame_stack::init();

    let mut areas = (*multiboot).memory_areas().unwrap();    
    let number_of_frames = areas.clone().fold(0, |n, area| {n + area.length / PAGE_SIZE}) as u32;

    // for memory > 1GB we need more p1 tables
    let p1_tables_required = (frame_stack::max_size(number_of_frames) / 4096 / 512) as u32;
    let mut last_mapped_p1_table = 0;

    // the first free physical frame (the 512 frames after kernel are used)
    let first_free_frame = Frame{
        number: stack_start_frame.number + 512,
    };

    for area in areas {
        for frame in range(area.base_addr >> 12, (area.base_addr+area.length) >> 12).map(
            |frame_number| Frame{number: frame_number as u32}) {
            if frame >= first_free_frame {
                if last_mapped_p1_table < p1_tables_required {
                    // add as p1 table
                    last_mapped_p1_table += 1;
                    map_to_p1_table(last_mapped_p1_table, frame);
                    // map p1 entries
                    for i in range(0, 512) {
                        map_p1_entries(last_mapped_p1_table, i, Frame{
                            number: stack_start_frame.number + last_mapped_p1_table*512 + i,
                        });
                    }
                } else {
                    // add as free
                    frame_stack::deallocate_frame(frame);
                }
            }
        }
    }
}

unsafe fn map_to_p1_table(p2_index: u32, to: Frame) { 
    *((0o177777_777_777_000_001_0000 + (p2_index as u64)*8) as *mut u64) = (to.number as u64 << 12) | 1;
}
unsafe fn map_p1_entries(p2_index: u32, p1_index: u32, to: Frame) {
    let entry = (0o177777_777_000_001_000_0000 | (p2_index as u64 << 12) | (p1_index as u64 * 8)) as *mut u64;
    *entry = (to.number as u64 << 12) | 3;
}