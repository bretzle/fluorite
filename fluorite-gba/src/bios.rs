use fluorite_common::num;
use fluorite_common::num::NumCast;
use fluorite_common::Ram;
use std::io::Read;
use std::mem::size_of;
use std::{cell::Cell, fs::File, path::Path};

const SIZE: usize = 16 * 1024;

const REPLACEMENT_BIOS: &[u8; SIZE] = include_bytes!("../../roms/gba_bios.bin");

pub struct Bios {
    data: Ram<SIZE>,
    latch: Cell<u32>,
}

impl Bios {
    pub fn new() -> Self {
        Self {
            data: Ram::new(*REPLACEMENT_BIOS),
            latch: Cell::new(0xE129F000),
        }
    }

    pub fn load<P: AsRef<Path>>(&mut self, path: P) {
        let mut file = File::open(path).unwrap();

        let bytes_read = file
            .read(self.data.data.as_mut())
            .expect("Failed to read file");

        assert!(bytes_read <= SIZE);
    }

    pub fn read_byte(&self, pc: u32, addr: u32) -> u8 {
        self.read(pc, addr)
    }

    pub fn read_half(&self, pc: u32, addr: u32) -> u16 {
        self.read(pc, addr)
    }

    pub fn read_word(&self, pc: u32, addr: u32) -> u32 {
        self.read(pc, addr)
    }

    fn read<T: NumCast>(&self, pc: u32, addr: u32) -> T {
        let addr = addr & !(size_of::<T>() as u32 - 1);

        if pc < 0x4000 {
            self.latch.set(self.data.read_fast(addr & !0x3))
        }

        num::cast(self.latch.get() >> (8 * (addr & 0x3))).unwrap()
    }
}
