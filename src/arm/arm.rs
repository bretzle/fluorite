use crate::arm::registers::Reg;
use crate::arm::Arm7tdmi;
use crate::arm::InstructionHandler;
use crate::arm::CONDITION_LUT;
use crate::io::{MemoryAccess, Sysbus};

include!(concat!(env!("OUT_DIR"), "/arm_lut.rs"));

impl Arm7tdmi {
    pub(super) fn fill_arm_instr_buffer(&mut self, bus: &mut Sysbus) {
        self.regs.pc &= !0x3;
        self.pipeline[0] = self.read::<u32>(bus, MemoryAccess::S, self.regs.pc & !0x3);
        self.regs.pc = self.regs.pc.wrapping_add(4);

        self.pipeline[1] = self.read::<u32>(bus, MemoryAccess::S, self.regs.pc & !0x3);
    }

    pub(super) fn emulate_arm_instr(&mut self, bus: &mut Sysbus) {
        let instr = self.pipeline[0];
        self.pipeline[0] = self.pipeline[1];
        self.regs.pc = self.regs.pc.wrapping_add(4);

        let condition =
            CONDITION_LUT[self.regs.get_flags() as usize | ((instr as usize >> 28) & 0xF)];

        if condition {
            ARM_LUT[((instr as usize) >> 16 & 0xFF0) | ((instr as usize) >> 4 & 0xF)](
                self, bus, instr,
            );
        } else {
            todo!()
            // self.instruction_prefetch::<u32>(bus, MemoryAccess::S);
        }
    }

    // ARM.3: Branch and Exchange (BX)
    fn branch_and_exchange(&mut self, bus: &mut Sysbus, instr: u32) {
        todo!()
    }

    // ARM.4: Branch and Branch with Link (B, BL)
    fn branch_branch_with_link<const LINK: bool>(&mut self, bus: &mut Sysbus, instr: u32) {
        let offset = instr & 0xFF_FFFF;
        let offset = if (offset >> 23) == 1 {
            0xFF00_0000 | offset
        } else {
            offset
        };

        self.instruction_prefetch::<u32>(bus, MemoryAccess::N);
        if LINK {
            self.regs.set_reg(Reg::R14, self.regs.pc.wrapping_sub(4))
        }
        self.regs.pc = self.regs.pc.wrapping_add(offset << 2);
        self.fill_arm_instr_buffer(bus);
    }

    // ARM.5: Data Processing
    fn data_proc<const I: bool, const S: bool>(&mut self, bus: &mut Sysbus, instr: u32) {
        todo!()
    }

    // ARM.6: PSR Transfer (MRS, MSR)
    fn psr_transfer<const I: bool, const P: bool, const L: bool>(
        &mut self,
        bus: &mut Sysbus,
        instr: u32,
    ) {
        todo!()
    }

    // ARM.7: Multiply and Multiply-Accumulate (MUL, MLA)
    fn mul_mula<const A: bool, const S: bool>(&mut self, bus: &mut Sysbus, instr: u32) {
        todo!()
    }

    // ARM.8: Multiply Long and Multiply-Accumulate Long (MULL, MLAL)
    fn mul_long<const U: bool, const A: bool, const S: bool>(
        &mut self,
        bus: &mut Sysbus,
        instr: u32,
    ) {
        todo!()
    }

    // ARM.9: Single Data Transfer (LDR, STR)
    fn single_data_transfer<
        const I: bool,
        const P: bool,
        const U: bool,
        const B: bool,
        const W: bool,
        const L: bool,
    >(
        &mut self,
        bus: &mut Sysbus,
        instr: u32,
    ) {
        todo!()
    }

    // ARM.10: Halfword and Signed Data Transfer (STRH,LDRH,LDRSB,LDRSH)
    fn halfword_and_signed_data_transfer<
        const P: bool,
        const U: bool,
        const I: bool,
        const W: bool,
        const L: bool,
        const S: bool,
        const H: bool,
    >(
        &mut self,
        bus: &mut Sysbus,
        instr: u32,
    ) {
        todo!()
    }

    // ARM.11: Block Data Transfer (LDM,STM)
    fn block_data_transfer<
        const P: bool,
        const U: bool,
        const S: bool,
        const W: bool,
        const L: bool,
    >(
        &mut self,
        bus: &mut Sysbus,
        instr: u32,
    ) {
        todo!()
    }

    // ARM.12: Single Data Swap (SWP)
    fn single_data_swap<const B: bool>(&mut self, bus: &mut Sysbus, instr: u32) {
        todo!()
    }

    // ARM.13: Software Interrupt (SWI)
    fn arm_software_interrupt(&mut self, bus: &mut Sysbus, instr: u32) {
        todo!()
    }

    // ARM.14: Coprocessor Data Operations (CDP)
    // ARM.15: Coprocessor Data Transfers (LDC,STC)
    // ARM.16: Coprocessor Register Transfers (MRC, MCR)
    fn coprocessor(&mut self, _bus: &mut Sysbus, _instr: u32) {
        unimplemented!("Coprocessor not implemented!");
    }

    // ARM.17: Undefined Instruction
    fn undefined_instr_arm(&mut self, _bus: &mut Sysbus, _instr: u32) {
        unimplemented!("ARM.17: Undefined Instruction not implemented!");
    }
}
