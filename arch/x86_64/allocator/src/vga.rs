/*
 * Copyright (c) 2014 Arcterus
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use core::prelude::*;
use core::intrinsics::transmute;
use core::iter;
use core::fmt;
use core::fmt::{Writer, Error};

#[allow(dead_code)]
pub enum Color {
   Black      = 0,
   Blue       = 1,
   Green      = 2,
   Cyan       = 3,
   Red        = 4,
   Magenta    = 5,
   Brown      = 6,
   LightGray  = 7,
   DarkGray   = 8,
   LightBlue  = 9,
   LightGreen = 10,
   LightCyan  = 11,
   LightRed   = 12,
   Pink       = 13,
   Yellow     = 14,
   White      = 15
}
impl Copy for Color {}

static SCREEN_ADDR: uint = 0xb8000;
static MAX_ROW: uint = 25;
static MAX_COLUMN: uint = 80;

pub struct ScreenWriter {
   row: uint,
   col: uint,
   foreground: Color,
   background: Color,
}

#[packed]
#[allow(dead_code)]
struct ScreenCharacter {
   character: u8,
   color: u8,
}
impl Copy for ScreenCharacter{}

static mut std_writer: ScreenWriter = ScreenWriter{row:0, col: 0, foreground: Color::Green, 
   background: Color::Black};

pub fn clear_screen() {
   unsafe{std_writer.clear_screen()};
}
pub fn print_args(args: fmt::Arguments) {
   unsafe{std_writer.write_fmt(args)};
}
pub fn print_err(args: fmt::Arguments) {
   unsafe{
      let foreground = std_writer.foreground;
      let background = std_writer.background;
      std_writer.foreground = Color::White;
      std_writer.background = Color::Red;
      print_args(args);
      std_writer.foreground = foreground;
      std_writer.background = background;
   }
}

pub fn set_foreground(color:Color) {
   unsafe{std_writer.foreground = color};
}
pub fn set_background(color:Color) {
   unsafe{std_writer.background = color};
}

impl ScreenCharacter {
   #[inline]
   fn new(character:u8, foreground:Color, background:Color) -> ScreenCharacter {
      ScreenCharacter{
         character:character, 
         color:((background as u8) << 4) + foreground as u8,
      }
   }
}

impl Writer for ScreenWriter {
   fn write_str(&mut self, s: &str) -> Result<(),Error> {
      for byte in s.as_bytes().iter() {
         self.print_byte(*byte);
      }
      Ok::<(), Error>(())
   }
}

impl ScreenWriter {
   #[inline]
   fn screen_char_at(pos:uint) -> &'static ScreenCharacter {
      unsafe{transmute::<uint,&ScreenCharacter>(SCREEN_ADDR + pos * 2)}
   }
   #[inline]
   fn mut_screen_char_at(pos:uint) -> &'static mut ScreenCharacter {
      unsafe{transmute::<uint,&mut ScreenCharacter>(SCREEN_ADDR + pos * 2)}
   }

   fn clear_screen(&mut self) {
      for line in iter::range(0, MAX_ROW) {
         self.clear_line(line);
      }
      self.row = 0;
      self.col = 0;
   }

   #[inline]
   fn print_byte(&mut self, byte: u8) {
      match byte {
         0x0a /* newline */ => self.add_line(),
         0x0d /* carriage return */ => self.col = 0,
         0x08 /* backspace */ => {
            if self.col == 0 && self.row != 0 {
               self.col = MAX_COLUMN - 1;
               self.row -= 1;
            } else if self.col != 0 {
               self.col -= 1;
            }
         }
         byte => {
            let pos = self.row * MAX_COLUMN + self.col;
            
            let screen_char = ScreenWriter::mut_screen_char_at(pos);
            *screen_char = ScreenCharacter::new(byte as u8, self.foreground, self.background);

            self.col += 1;
            if self.col == MAX_COLUMN {
               self.add_line();
            }
         }
      }      
   }


   fn clear_line(&mut self, row: uint) {
      let c = self.col;
      let r = self.row;
      self.col = 0;
      self.row = row;
      self.clear_rem_line();
      self.row = r;
      self.col = c;
   }

   fn clear_rem_line(&mut self) {
      let rpos = self.row * MAX_COLUMN;
      for i in iter::range(self.col, MAX_COLUMN) {
         let pos = rpos + i;
         let screen_char = ScreenWriter::mut_screen_char_at(pos);
         *screen_char = ScreenCharacter::new(' ' as u8, self.foreground, self.background);
      }
   }

   fn add_line(&mut self) {
      self.clear_rem_line();
      self.col = 0;
      self.row += 1;
      if self.row == MAX_ROW {
         self.row -= 1;
         self.shift_rows_up();
      }
   }

   fn shift_rows_up(&mut self) {
      for r in iter::range(0, MAX_ROW-1) {
         for c in iter::range(0, MAX_COLUMN) {
            let new_pos = r * MAX_COLUMN + c;
            let old_pos = (r+1) * MAX_COLUMN + c;

            let new_field = ScreenWriter::mut_screen_char_at(new_pos);
            let old_field = ScreenWriter::screen_char_at(old_pos);
            *new_field = *old_field;
         }
      }
      self.clear_line(MAX_ROW - 1);
   }
}