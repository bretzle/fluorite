use crate::{
    cpu::Arm7tdmi,
    memory::MemoryInterface,
    registers::{CpuMode, CpuState},
};

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
    pub fn exception(&mut self, e: Exception, lr: u32) {
        use Exception::*;
        let (new_mode, irq_disable, fiq_disable) = match e {
            Reset => (CpuMode::Supervisor, true, true),
            UndefinedInstruction => (CpuMode::Undefined, false, false),
            SoftwareInterrupt => (CpuMode::Supervisor, true, false),
            DataAbort => (CpuMode::Abort, false, false),
            PrefatchAbort => (CpuMode::Abort, false, false),
            Reserved => panic!("Cpu reserved exception"),
            Irq => (CpuMode::Irq, true, false),
            Fiq => (CpuMode::Fiq, true, true),
        };

        println!("[exception] {:?} lr={:x} new_mode={:?}", e, lr, new_mode);

        let new_bank = new_mode.bank_index();
        self.banks.spsr_bank[new_bank] = self.cspr;
        self.banks.gpr_banked_r14[new_bank] = lr;
        self.change_mode(self.cspr.mode(), new_mode);

        // Set appropriate CPSR bits
        self.cspr.set_state(CpuState::ARM);
        self.cspr.set_mode(new_mode);
        if irq_disable {
            self.cspr.set_irq_disable(true);
        }
        if fiq_disable {
            self.cspr.set_fiq_disable(true);
        }

        // Set PC to vector address
        self.pc = e as u32;
        self.reload_pipeline_arm();
    }

    pub fn irq(&mut self) {
        todo!()
    }

    pub fn software_interrupt(&mut self, lr: u32, _cmt: u32) {
        self.exception(Exception::SoftwareInterrupt, lr);
    }
}
