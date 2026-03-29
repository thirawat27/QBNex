use std::cell::RefCell;

pub const DOS_MEMORY_SIZE: usize = 1_048_576;

pub const VGA_VIDEO_RAM_START: usize = 0xA0000;
pub const VGA_VIDEO_RAM_END: usize = 0xAFFFF;
pub const TEXT_VIDEO_RAM_START: usize = 0xB8000;
pub const TEXT_VIDEO_RAM_END: usize = 0xBFFFF;

#[derive(Clone)]
pub struct DosMemory {
    buffer: RefCell<Vec<u8>>,
}

impl DosMemory {
    pub fn new() -> Self {
        Self {
            buffer: RefCell::new(vec![0u8; DOS_MEMORY_SIZE]),
        }
    }

    pub fn absolute_address(segment: u16, offset: u16) -> usize {
        ((segment as usize) * 16) + (offset as usize)
    }

    pub fn read_byte(&self, addr: usize) -> u8 {
        if addr < DOS_MEMORY_SIZE {
            self.buffer.borrow()[addr]
        } else {
            0
        }
    }

    pub fn write_byte(&self, addr: usize, value: u8) {
        if addr < DOS_MEMORY_SIZE {
            self.buffer.borrow_mut()[addr] = value;
        }
    }

    pub fn read_word(&self, addr: usize) -> u16 {
        if addr + 1 < DOS_MEMORY_SIZE {
            let low = self.read_byte(addr) as u16;
            let high = self.read_byte(addr + 1) as u16;
            (high << 8) | low
        } else {
            0
        }
    }

    pub fn write_word(&self, addr: usize, value: u16) {
        self.write_byte(addr, (value & 0xFF) as u8);
        self.write_byte(addr + 1, ((value >> 8) & 0xFF) as u8);
    }

    pub fn read_dword(&self, addr: usize) -> u32 {
        if addr + 3 < DOS_MEMORY_SIZE {
            let low_word = self.read_word(addr) as u32;
            let high_word = self.read_word(addr + 2) as u32;
            (high_word << 16) | low_word
        } else {
            0
        }
    }

    pub fn write_dword(&self, addr: usize, value: u32) {
        self.write_word(addr, (value & 0xFFFF) as u16);
        self.write_word(addr + 2, ((value >> 16) & 0xFFFF) as u16);
    }

    pub fn get_vga_buffer(&self) -> Vec<u8> {
        let mut result = Vec::with_capacity(VGA_VIDEO_RAM_END - VGA_VIDEO_RAM_START);
        let buf = self.buffer.borrow();
        for i in VGA_VIDEO_RAM_START..VGA_VIDEO_RAM_END {
            result.push(buf[i]);
        }
        result
    }

    pub fn get_text_buffer(&self) -> Vec<u8> {
        let mut result = Vec::with_capacity(TEXT_VIDEO_RAM_END - TEXT_VIDEO_RAM_START);
        let buf = self.buffer.borrow();
        for i in TEXT_VIDEO_RAM_START..TEXT_VIDEO_RAM_END {
            result.push(buf[i]);
        }
        result
    }

    pub fn peek(&self, segment: u16, offset: u16) -> u8 {
        let addr = Self::absolute_address(segment, offset);
        self.read_byte(addr)
    }

    pub fn poke(&self, segment: u16, offset: u16, value: u8) {
        let addr = Self::absolute_address(segment, offset);
        self.write_byte(addr, value);
    }

    pub fn clear(&self) {
        self.buffer.borrow_mut().fill(0);
    }
}

impl Default for DosMemory {
    fn default() -> Self {
        Self::new()
    }
}
