use fluorite_arm::{Access, Arm7tdmi, SysBus};
use fluorite_common::ptr::WeakPointer;
use std::path::Path;

use crate::{
    bios::Bios,
    consts::{HEIGHT, WIDTH},
    gamepak::Gamepak,
};

pub struct Gba {
    pub cpu: Arm7tdmi<Self>,

    bios: Bios,
    gamepak: Gamepak,

    // temp
    pixels: [u16; WIDTH * HEIGHT],
}

impl Gba {
    pub fn new() -> Self {
        Self {
            cpu: Arm7tdmi::new(),
            bios: Bios::new(),
            gamepak: Gamepak::new(),
            pixels: [0; WIDTH * HEIGHT],
        }
    }

    pub fn init(&mut self) {
        let ptr = WeakPointer::new(self);
        self.cpu.connect(ptr);
    }

    pub fn reset(&mut self) -> bool {
        self.cpu.reset();

        self.cpu.init();

        self.gamepak.ready()
    }

    pub fn get_pixels(&self) -> &[u16] {
        &self.pixels
    }

    pub fn load_rom<P: AsRef<Path>>(&mut self, path: P) {
        self.gamepak.load(Some(path.as_ref()), None)
    }
}

impl SysBus for Gba {
    fn read_byte(&self, addr: u32, access: Access) -> u8 {
        todo!()
    }

    fn read_half(&self, addr: u32, access: Access) -> u16 {
        todo!()
    }

    fn read_word(&self, addr: u32, access: Access) -> u32 {
        todo!()
    }

    fn write_byte(&mut self, addr: u32, byte: u8, access: Access) {
        todo!()
    }

    fn write_half(&mut self, addr: u32, half: u16, access: Access) {
        todo!()
    }

    fn write_word(&mut self, addr: u32, word: u32, access: Access) {
        todo!()
    }
}
