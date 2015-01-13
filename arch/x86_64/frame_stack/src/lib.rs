#![no_std]
#![feature(globs)]

extern crate core;
extern crate spinlock;

use core::prelude::*;
use core::intrinsics::{offset, size_of};
use core::cmp::Ordering;
use spinlock::Spinlock;

pub struct Frame {
    pub number: u32,
}

impl Copy for Frame {}

impl PartialEq for Frame {
    fn eq(&self, other: &Frame) -> bool {
        self.number.eq(&other.number)
    }
}
impl PartialOrd for Frame {
    fn partial_cmp(&self, other: &Frame) -> Option<Ordering> {
        Some(self.number.cmp(&other.number))
    }
}

const STACK_POINTER : *mut Spinlock<FrameStack> = 0o_001_000_000_0000 as *mut Spinlock<FrameStack>; //10MB

struct FrameStack {
    first: *const Frame,
    length: u32, //enough for up to 16TB memory
}

pub unsafe fn init() {
    (*STACK_POINTER) = Spinlock::new(FrameStack {
        first: offset(STACK_POINTER as *const Spinlock<FrameStack>, 1) as *const Frame, 
        length: 0,
    });
}

pub fn length() -> u32 {
    unsafe{(*STACK_POINTER).lock().length}
}

/// returns maximal size of frame stack for given number of frames
pub unsafe fn max_size(number_of_frames: u32) -> u64 {
    size_of::<FrameStack>() as u64 + number_of_frames as u64 * size_of::<Frame>() as u64
}

pub fn allocate_frame() -> Option<Frame> {
    unsafe{(*STACK_POINTER).lock().pop()}
}

pub fn deallocate_frame(frame: Frame) {
    unsafe{(*STACK_POINTER).lock().push(frame)}
}

impl FrameStack {
    unsafe fn push(&mut self, frame: Frame) {
        let last = offset(self.first, self.length as int);
        *(last as *mut Frame) = frame;
        self.length += 1;
    }

    fn pop(&mut self) -> Option<Frame> {
        if self.length == 0 {
            None
        } else {
            self.length -= 1;
            Some(unsafe{*offset(self.first, self.length as int)})
        }
    }
}