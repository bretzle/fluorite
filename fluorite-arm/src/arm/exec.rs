use crate::{
    arm::ArmInstruction,
    cpu::{Arm7tdmi, CpuAction},
    memory::MemoryInterface,
    InstructionDecoder,
};
use fluorite_common::BitIndex;

impl<Memory: MemoryInterface> Arm7tdmi<Memory> {
    pub(crate) fn execute_arm(&mut self, inst: u32) -> CpuAction {
        use crate::arm::ArmFormat::*;
        let decoded = ArmInstruction::decode(inst, self.pc_arm());

        let func = match decoded.fmt {
            BranchExchange => Self::bx,
            BranchLink => Self::b_bl,
            SoftwareInterrupt => Self::swi,
            Multiply => Self::mul_mla,
            MultiplyLong => Self::mull_mlal,
            SingleDataTransfer => Self::ldr_str,
            HalfwordDataTransferRegOffset => Self::ldr_str_hs_reg,
            HalfwordDataTransferImmediateOffset => Self::ldr_str_hs_imm,
            DataProcessing => Self::data_processing,
            BlockDataTransfer => Self::ldm_stm,
            SingleDataSwap => Self::exec_arm_swp,
            MoveFromStatus => Self::mrs,
            MoveToStatus => Self::transfer_to_status,
            MoveToFlags => unreachable!(), // what is this???
            Undefined => Self::undefined,
        };

        func(self, inst)
    }

    fn mul_mla(&mut self, inst: u32) -> CpuAction {
        let update_flags = inst.bit(20);
        let accumulate = inst.bit(21);

        todo!()
    }

    fn mull_mlal(&mut self, inst: u32) -> CpuAction {
        let update_flags = inst.bit(20);
        let accumulate = inst.bit(21);
        let u_flag = inst.bit(22);

        todo!()
    }

    fn exec_arm_swp(&mut self, inst: u32) -> CpuAction {
        let byte = inst.bit(22);
        todo!()
    }

    fn bx(&mut self, inst: u32) -> CpuAction {
        todo!()
    }

    fn swi(&mut self, inst: u32) -> CpuAction {
        todo!()
    }

    fn b_bl(&mut self, inst: u32) -> CpuAction {
        let link = inst.bit(24);

        todo!()
    }

    fn ldr_str_hs_imm(&mut self, inst: u32) -> CpuAction {
        let hs = (inst & 0b1100000) >> 5;
        let load = inst.bit(20);
        let writeback = inst.bit(21);
        let pre_index = inst.bit(24);
        let add = inst.bit(23);
        todo!()
    }

    fn ldr_str_hs_reg(&mut self, inst: u32) -> CpuAction {
        let hs = (inst as u8 & 0b1100000) >> 5;
        let load = inst.bit(20);
        let writeback = inst.bit(21);
        let add = inst.bit(23);
        let pre_index = inst.bit(24);

        todo!()
    }

    fn data_processing(&mut self, inst: u32) -> CpuAction {
        let op = inst.bit_range(21..25);
        let imm = inst.bit(25);
        let set_flags = inst.bit(20);
        let shift_by_reg = inst.bit(4);

        todo!()
    }

    fn ldm_stm(&mut self, i: u32) -> CpuAction {
        let load = i.bit(20);
        let writeback = i.bit(21);
        let flag_s = i.bit(22);
        let add = i.bit(23);
        let pre_index = i.bit(24);

        todo!()
    }

    fn ldr_str(&mut self, i: u32) -> CpuAction {
        let load = i.bit(20);
        let writeback = i.bit(21);
        let byte = i.bit(22);
        let add = i.bit(23);
        let pre_index = i.bit(24);
        let shift = i.bit(25);
        let bs_op = i.bit_range(5..7) as u8;
        let shift_by_reg = i.bit(4);

        todo!()
    }

    fn mrs(&mut self, i: u32) -> CpuAction {
        let spsr_flag = i.bit(22);

        todo!()
    }

    fn transfer_to_status(&mut self, i: u32) -> CpuAction {
        let imm = i.bit(25);
        let spsr_flag = i.bit(22);

        todo!()
    }

    fn undefined(&mut self, inst: u32) -> CpuAction {
        panic!(
            "executing undefined arm instruction {:08x} at @{:08x}",
            inst,
            self.pc_arm()
        )
    }
}
