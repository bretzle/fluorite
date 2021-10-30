use crate::{cpu::Arm7tdmi, memory::MemoryInterface};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Exception {
    Reset = 0x00,
    UndefinedInstruction = 0x04,
    SoftwareInterrupt = 0x08,
    PrefatchAbort = 0x0c,
    DataAbort = 0x10,
    Reserved = 0x14,
    Irq = 0x18,
    Fiq = 0x1c,
}

impl<Memory: MemoryInterface> Arm7tdmi<Memory> {
    pub fn exception(&mut self, _e: Exception, _lr: u32) {
        todo!()
    }

    pub fn irq(&mut self) {
        todo!()
    }

    pub fn software_interrupt(&mut self, _lr: u32) {
        todo!()
    }
}
