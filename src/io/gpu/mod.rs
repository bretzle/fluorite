use self::{debug::DebugSpecification, registers::Dispcnt};
use crate::gba::{DebugSpec, Pixels, HEIGHT, WIDTH};
use std::sync::{Arc, Mutex};

use super::scheduler::Scheduler;

pub mod debug;
mod registers;

pub struct Gpu {
    // Registers
    dispcnt: Dispcnt,

    pub vram: Box<[u8]>,
}

impl Gpu {
    pub fn new() -> (Self, Pixels, DebugSpec) {
        let pixels = Arc::new(Mutex::new(vec![0; WIDTH * HEIGHT]));
        let debug = Arc::new(Mutex::new(DebugSpecification::new()));

        let gpu = Self {
            dispcnt: Dispcnt::new(),
            vram: vec![0; 0x18000].into_boxed_slice(),
        };

        (gpu, pixels, debug)
    }

    pub fn write_register(&mut self, scheduler: &mut Scheduler, addr: u32, val: u8) {
        assert_eq!(addr >> 12, 0x04000);

        match addr & 0xFFF {
            0x000 => self.dispcnt.write(0, val),
            0x001 => self.dispcnt.write(1, val),
            _ => panic!("Ignoring GPU Write 0x{addr:08X} = 0x{val:02X}"),
        }
    }

	#[inline]
    pub fn parse_vram_addr(addr: u32) -> u32 {
        let addr = addr & 0x1FFFF;
        if addr < 0x10000 {
            addr
        } else {
            addr & 0x17FFF
        }
    }
}
