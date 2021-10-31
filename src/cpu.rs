use std::any::Any;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use fluorite_arm::cpu::Arm7tdmi;
use fluorite_common::Shared;

use crate::sysbus::SysBus;

static BIOS: &[u8] = include_bytes!("../roms/gba_bios.bin");
static ROM: &[u8] = include_bytes!("../roms/first-1.bin");

pub struct GbaCpu {
    cpu: Arm7tdmi<SysBus>,
    sysbus: Shared<SysBus>,
    io: (),
    scheduler: (),
}

impl GbaCpu {
    pub fn new() -> Self {
        let sysbus = Shared::new(SysBus::new(BIOS, ROM));
        let cpu = Arm7tdmi::new(sysbus.clone());

        Self {
            cpu,
            sysbus,
            io: (),
            scheduler: (),
        }
    }

    pub fn run(&mut self) {
		loop {
			self.cpu.step();
		}
	}

	pub fn skip_bios(&mut self) {
		self.cpu.skip_bios();
	}
}
