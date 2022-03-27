use crate::{arm::Arm7tdmi, consts::CLOCKS_PER_FRAME, io::Sysbus};
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
            cpu: Arm7tdmi::new(false, &mut bus),
            bus,
            next_frame_cycle: 0,
        }
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

    pub fn get_pixels(&self) -> &[u16] {
        &self.bus.gpu.pixels
    }

    pub fn load_rom<P: AsRef<Path>>(&mut self, _path: P) {
        todo!()
    }
}
