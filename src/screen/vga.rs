use core::ptr;

use spin::Mutex;

use crate::{mem::filler, processor::x86::io, text::cp437};

const VGA_BUFFER: *mut u8 = 0xb8000 as *mut u8;
#[allow(dead_code)]
const VGA_WIDTH: u16 = 80;
const VGA_HEIGHT: u16 = 25;
const VGA_CONTROL_REGISTER_PORT: u16 = 0x3D4;
const VGA_DATA_REGISTER_PORT: u16 = 0x3D5;
const COLOR: u8 = 0x0F;
const BLANK_CHARACTER: u16 = 0x20 | ((COLOR as u16) << 8);

pub static VGA_SCREEN: Mutex<VGAScreen> = Mutex::new(VGAScreen { x: 0, y: 0 });

pub struct VGAScreen {
    /// X-axis virtual cursor position.
    x: u16,

    /// Y-asis virtual cursor position.
    y: u16,
}

impl VGAScreen {
    pub fn write_string(&mut self, str: &str) {
        for character in str.chars() {
            self.write_char(character);
        }
    }

    pub fn write_char(&mut self, character: char) {
        let character_byte = cp437::normalize_to_cp437(character);

        match character_byte {
            b'\n' => {
                self.x = 0;
                self.y += 1;
            }

            b'\r' => {
                self.x = 0;
            }

            b'\t' => {
                // Advance to next multiple of 8
                // !(8 - 1) = 1111 1000
                self.x = self.x & !(8 - 1);
            }

            character_byte => {
                let offset = (2 * (self.y * 80 + self.x)) as usize;

                unsafe {
                    *VGA_BUFFER.add(offset) = character_byte;
                    *VGA_BUFFER.add(offset + 1) = COLOR;
                }

                self.x += 1;
            }
        }

        if self.x >= 80 {
            self.x = 0;
            self.y += 1;
        }

        self.scroll_if_needed();
        self.update_hardware_cursor();
    }

    pub fn clear_screen(&mut self) {
        for line in 0..25 {
            unsafe {
                filler::memsetw(VGA_BUFFER.add(line * 80) as *mut u16, BLANK_CHARACTER, 80);
            }
        }

        self.reset();
    }

    #[inline(always)]
    fn reset(&mut self) {
        self.x = 0;
        self.y = 0;

        self.update_hardware_cursor();
    }

    #[inline(always)]
    fn scroll_if_needed(&mut self) {
        if self.y < VGA_HEIGHT {
            return;
        }

        let hidden_line_count = (self.y - VGA_HEIGHT + 1) as usize;

        unsafe {
            // Move line to top.
            ptr::copy_nonoverlapping(
                VGA_BUFFER.add(hidden_line_count * 80),
                VGA_BUFFER,
                (25 - hidden_line_count) * 80 * 2,
            );

            // Clear last line.
            filler::memsetw(
                VGA_BUFFER.add(hidden_line_count + (25 - hidden_line_count)) as *mut u16,
                BLANK_CHARACTER,
                80,
            );
        }
    }

    fn update_hardware_cursor(&self) {
        let index = self.y * 80 + self.x;

        unsafe {
            // Write high byte.
            io::outportb(VGA_CONTROL_REGISTER_PORT, 14);
            io::outportb(VGA_DATA_REGISTER_PORT, (index >> 8) as u8);

            // Write low byte.
            io::outportb(VGA_CONTROL_REGISTER_PORT, 15);
            io::outportb(VGA_DATA_REGISTER_PORT, index as u8);
        }
    }
}
