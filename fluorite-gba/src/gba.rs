use crate::{arm::Arm7tdmi, io::Sysbus, AudioInterface};
use fluorite_common::{flume::Receiver, EasyCell};
use std::path::Path;

pub struct Gba {
    pub cpu: Arm7tdmi,
    pub bus: Sysbus,
    pub next_frame_cycle: usize,
}

pub type Pixels = Vec<u16>;

pub static AUDIO_DEVICE: EasyCell<&mut dyn AudioInterface> = EasyCell::new();

impl Gba {
    pub fn new(rx: Receiver<(u16, bool)>) -> Self {
        let mut bus = Sysbus::new(rx);

        Self {
            cpu: Arm7tdmi::new(true, &mut bus),
            bus,
            next_frame_cycle: 0,
        }
    }

    pub fn load_audio(device: *mut dyn AudioInterface) {
        AUDIO_DEVICE.init(|| unsafe { &mut *device });
    }

    pub fn reset(&mut self) {
        self.bus.reset();
        self.cpu.reset(true, &mut self.bus);
        self.next_frame_cycle = 0;
    }

    pub fn run(&mut self, cycles: usize) {
        self.next_frame_cycle += cycles;
        self.bus.poll_keypad_updates();
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
        // self.bus.rom = std::fs::read(path).unwrap().into_boxed_slice();
        self.bus.gamepak.load(Some(path.as_ref()), None)
    }
}
