use crate::{arm::Arm7tdmi, io::Sysbus};
use std::path::Path;

pub struct Gba {
    pub cpu: Arm7tdmi,
    pub bus: Sysbus,
    pub next_frame_cycle: usize,
}

pub type Pixels = Vec<u16>;

impl Gba {
    pub fn new(bios: Vec<u8>, rom: Vec<u8>) -> Self {
        let mut bus = Sysbus::new(bios, rom);

        Self {
            cpu: Arm7tdmi::new(true, &mut bus),
            bus,
            next_frame_cycle: 0,
        }
    }

    pub fn reset(&mut self) {
        self.bus.reset();
        self.cpu.reset(true, &mut self.bus);
        self.next_frame_cycle = 0;
    }

    pub fn run(&mut self, cycles: usize) {
        self.next_frame_cycle += cycles;

        while self.bus.get_cycle() < self.next_frame_cycle {
            self.bus.run_dma();
            self.cpu.handle_irq(&mut self.bus);
            self.cpu.emulate_instr(&mut self.bus);
        }
    }

    pub fn get_pixels(&self) -> &[u16] {
        &self.bus.gpu.pixels
    }

    pub fn load_rom<P: AsRef<Path>>(&mut self, path: P) {
        self.bus.rom = std::fs::read(path).unwrap().into_boxed_slice();
    }
}
