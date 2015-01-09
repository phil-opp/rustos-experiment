#![no_std]
#![allow(dead_code)]

extern crate core;
extern crate frame_stack;

use core::prelude::*;
use self::frame_stack::{Frame, allocate_frame};

// for macros
mod std {
    pub use core::*;
}

#[macro_use]
mod macros;

#[macro_use]
mod bitflags;

/* for testing */
mod vga;

const PAGE_SIZE: u64 = 4096;

struct VirtualAddress(*const u8);
impl Copy for VirtualAddress{}

#[derive(PartialEq)]
struct Page {
    number: usize,
}
impl Copy for Page{}
struct PageIter(Page);

struct PageTablePage(Page);
struct PageTableField(*const u64);

bitflags! {
    flags PageTableFieldFlags: u64 {
        const NOT_FREE = 1 << 11,
        const PRESENT =         NOT_FREE.bits | 1 << 0,
        const WRITABLE =        NOT_FREE.bits | 1 << 1,
        const USER_ACCESSIBLE = NOT_FREE.bits | 1 << 2,
        const WRITE_THROUGH =   NOT_FREE.bits | 1 << 3,
        const NO_CACHE =        NOT_FREE.bits | 1 << 4,
        const ACCESSED =        NOT_FREE.bits | 1 << 5,
        const DIRTY =           NOT_FREE.bits | 1 << 6,
        const OTHER1 =          NOT_FREE.bits | 1 << 9,
        const OTHER2 =          NOT_FREE.bits | 1 << 10,
        const NO_EXECUTE =      NOT_FREE.bits | 1 << 63,
    }
}

struct Allocator {
    current_page: Page,
    next_byte: VirtualAddress,
}

static mut allocator: Option<Allocator> = None;
// first allocated address starts on second P4-Page
static FIRST_PAGE : Page = Page {
    number: 0o_001_000_000_000,
};

pub unsafe fn allocate(size: usize, align: usize) -> *const u8 {
    if allocator.is_none() {
        FIRST_PAGE.map_to_new_frame();
        allocator = Some(Allocator {
            next_byte: FIRST_PAGE.start_address(),
            current_page: FIRST_PAGE,
        });
    }
    allocator.as_mut().expect("allocator must be initialized").allocate(size, align).0
}

pub unsafe fn deallocate(_ptr: *mut u8, _old_size: usize, _align: usize) {
    //print!("start: {:x}, size: {:x}, align: {:x}\n", _ptr as usize, _old_size, _align);
}

impl Allocator {
    unsafe fn allocate(&mut self, size: usize, align: usize) -> VirtualAddress {
        let addr = self.next_byte.0 as usize;

        //align
        if align > 0 && addr % align != 0 {
            self.next_byte = VirtualAddress((addr + align - (addr % align)) as *const u8);
        }

        //map unmapped pages if allocation is on new pages
        let end_page = VirtualAddress((addr + size - 1) as *const u8).page();
        for page in self.current_page.next_pages().take(
                end_page.number - self.current_page.number) {
            page.map_to_new_frame();
        }

        //allocate
        let start = self.next_byte;
        self.next_byte = VirtualAddress((addr + size) as *const u8);
        self.current_page = end_page;
       
        // DEBUGGING: zero allocated bytes
        for i in (0..size) {
            *((start.0 as usize + i) as *mut u8) = 0;
        }

        start
    }
}

impl Page {
    fn from_address(address: &VirtualAddress) -> Page {
        Page {
            number: address.0 as usize >> 12,
        }
    }

    fn start_address(&self) -> VirtualAddress {
        if self.number >= 0o400_000_000_000 {
            //sign extension
            VirtualAddress(((self.number << 12) | 0o177777_000_000_000_000_0000) as *const u8)
        } else {
            VirtualAddress((self.number << 12) as *const u8)
        }
    }

    fn p4_index(&self) -> usize {(self.number >> 27) & 0o777}
    fn p3_index(&self) -> usize {(self.number >> 18) & 0o777}
    fn p2_index(&self) -> usize {(self.number >> 9) & 0o777}
    fn p1_index(&self) -> usize {(self.number >> 0) & 0o777}

    fn p4_page(&self) -> PageTablePage {
        PageTablePage(Page {
            number: 0o_777_777_777_777,
        })
    }
    fn p3_page(&self) -> PageTablePage {
        PageTablePage(Page {
            number: 0o_777_777_777_000 | self.p4_index(),
        })
    }
    fn p2_page(&self) -> PageTablePage {
        PageTablePage(Page {
            number: 0o_777_777_000_000 | (self.p4_index() << 9) | self.p3_index(),
        })
    }
    fn p1_page(&self) -> PageTablePage {
        PageTablePage(Page {
            number: 0o_777_000_000_000 | (self.p4_index() << 18) | (self.p3_index() << 9)
                | self.p2_index(),
        })
    }

    unsafe fn map_to_new_frame(&self) {
        let p4_field = self.p4_page().field(self.p4_index());
        if p4_field.is_free() {
            p4_field.set(allocate_frame().expect("no frame allocated"), PRESENT | WRITABLE);
            self.p3_page().zero();
        }
        let p3_field = self.p3_page().field(self.p3_index());
        if p3_field.is_free() {
            p3_field.set(allocate_frame().expect("no frame allocated"), PRESENT | WRITABLE);
            self.p2_page().zero();
        }
        let p2_field = self.p2_page().field(self.p2_index());
        if p2_field.is_free() {
            p2_field.set(allocate_frame().expect("no frame allocated"), PRESENT | WRITABLE);
            self.p1_page().zero();
        }
        let p1_field = self.p1_page().field(self.p1_index());
        assert!(p1_field.is_free());
        p1_field.set(allocate_frame().expect("no frame allocated"), PRESENT | WRITABLE);
    }

    unsafe fn zero(&self) {
        let page = self.start_address().0 as *mut [u64; (PAGE_SIZE/64) as usize];
        *page = [0; (PAGE_SIZE/64) as usize];
    }

    fn next_pages(self) -> PageIter {
        PageIter(self)
    }
}

impl Iterator for PageIter {
    type Item = Page;

    fn next(&mut self) -> Option<Page> {
        self.0.number += 1;
        Some(self.0)
    }
}

impl PageTablePage {
    fn field(&self, index: usize) -> PageTableField {
        //print!("index: {} pointer: {:o}\n", index, self.0.start_address().0 as usize + (index * 8));
        PageTableField((self.0.start_address().0 as usize + (index * 8)) as *const u64)
    }
    fn zero(&self) {unsafe{self.0.zero()}}
}

impl VirtualAddress {
    fn page(&self) -> Page {
        Page::from_address(self)
    }

    fn page_offset(&self) -> u32 {
        self.0 as u32 & 0xfff
    }
}

impl PageTableField {

    unsafe fn is(&self, flags: PageTableFieldFlags) -> bool {
        
        //print!("{:o}\n", self.0 as usize);
        PageTableFieldFlags::from_bits_truncate(*(self.0)).contains(flags)
    }

    unsafe fn add_flag(&self, flags: PageTableFieldFlags) {
        *(self.0 as *mut u64) |= flags.bits;
    }

    unsafe fn remove_flag(&self, flags: PageTableFieldFlags) {
        *(self.0 as *mut u64) &= !flags.bits;
    }

    unsafe fn is_free(&self) -> bool {
        !self.is(NOT_FREE)
    }

    unsafe fn pointed_frame(&self) -> Frame {
        Frame {
            number: (((*self.0) & 0x000fffff_fffff000) >> 12) as u32,
        }
    }

    unsafe fn set(&self, frame: Frame, flags: PageTableFieldFlags) {
        let f = self.0 as *mut u64;
        *f = (((frame.number as u64) << 12) & 0x000fffff_fffff000) | flags.bits();
    }
}
