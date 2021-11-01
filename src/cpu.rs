use std::any::Any;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use fluorite_arm::cpu::Arm7tdmi;
use fluorite_common::Shared;

use crate::iodev::IoDevices;
use crate::sysbus::SysBus;

static BIOS: &[u8] = include_bytes!("../roms/gba_bios.bin");
static ROM: &[u8] = include_bytes!("../roms/beeg.bin");

pub struct Gba {
    cpu: Arm7tdmi<SysBus>,
    sysbus: Shared<SysBus>,
    io: Shared<IoDevices>,
    scheduler: (),
}

impl Gba {
    pub fn new() -> Self {
        let io = Shared::new(IoDevices::new());
        let sysbus = Shared::new(SysBus::new(BIOS, ROM, &io));
        let cpu = Arm7tdmi::new(sysbus.clone());

        Self {
            cpu,
            sysbus,
            io,
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
