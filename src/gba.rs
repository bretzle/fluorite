use crate::{
    arm::Arm7tdmi,
    io::{gpu::debug::DebugSpecification, Sysbus},
};
use std::sync::{Arc, Mutex};

pub const WIDTH: usize = 240;
pub const HEIGHT: usize = 160;
pub const SCALE: usize = 4;
pub const CLOCKS_PER_FRAME: usize = 280896;
pub const CLOCK_FREQ: usize = 1 << 24;

pub struct Gba {
    pub cpu: Arm7tdmi,
    pub bus: Sysbus,
    pub next_frame_cycle: usize,
}

pub type Pixels = Arc<Mutex<Vec<u16>>>;
pub type DebugSpec = Arc<Mutex<DebugSpecification>>;

impl Gba {
    pub fn new(bios: Vec<u8>, rom: Vec<u8>) -> (Self, Pixels, DebugSpec) {
        let (mut bus, pixels, debug) = Sysbus::new(bios, rom);

        let gba = Self {
            cpu: Arm7tdmi::new(true, &mut bus),
            bus,
            next_frame_cycle: 0,
        };

        (gba, pixels, debug)
    }

    pub fn emulate_frame(&mut self) {
        self.bus.poll_keypad_updates();
        self.next_frame_cycle += CLOCKS_PER_FRAME;
        while self.bus.get_cycle() < self.next_frame_cycle {
            self.bus.run_dma();
            self.cpu.handle_irq(&mut self.bus);
            self.cpu.emulate_instr(&mut self.bus);
        }
    }
}