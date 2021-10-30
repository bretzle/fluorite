use crate::{
    alu::{AluOpCode, ShiftRegisterBy, ShiftedRegister},
    arm::ArmInstruction,
    cpu::{Arm7tdmi, CpuAction},
    memory::MemoryInterface,
    registers::{CpuMode, CpuState},
    Addr, InstructionDecoder, REG_PC,
};
use fluorite_common::BitIndex;
use num_traits::FromPrimitive;

use super::{ArmDecodeHelper, ArmHalfwordTransferType};

impl<Memory: MemoryInterface> Arm7tdmi<Memory> {
    pub(crate) fn execute_arm(&mut self, inst: u32) -> CpuAction {
        use crate::arm::ArmFormat::*;
        let decoded = ArmInstruction::decode(inst, self.pc_arm());

        println!(
            "{:8x}:\t{:08x} \t{}",
            self.pc_arm(),
            decoded.get_raw(),
            decoded
        );

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
        let _update_flags = inst.bit(20);
        let _accumulate = inst.bit(21);

        todo!()
    }

    fn mull_mlal(&mut self, inst: u32) -> CpuAction {
        let _update_flags = inst.bit(20);
        let _accumulate = inst.bit(21);
        let _u_flag = inst.bit(22);

        todo!()
    }

    fn exec_arm_swp(&mut self, inst: u32) -> CpuAction {
        let _byte = inst.bit(22);
        todo!()
    }

    fn bx(&mut self, _inst: u32) -> CpuAction {
        todo!()
    }

    fn swi(&mut self, _inst: u32) -> CpuAction {
        todo!()
    }

    fn b_bl(&mut self, inst: u32) -> CpuAction {
        let link = inst.bit(24);

        if link {
            todo!();
        }

        self.pc = (self.pc as i32).wrapping_add(inst.branch_offset()) as u32 & !1;

        self.reload_pipeline_arm();

        CpuAction::PipelineFlushed
    }

    fn ldr_str_hs_imm(&mut self, inst: u32) -> CpuAction {
        let hs = (inst as u8 & 0b1100000) >> 5;
        let load = inst.bit(20);
        let writeback = inst.bit(21);
        let pre_index = inst.bit(24);
        let add = inst.bit(23);

        let offset = (inst.bit_range(8..12) << 4) + inst.bit_range(0..4);
        self.ldr_str_common(inst, offset, hs, load, writeback, pre_index, add)
    }

    fn ldr_str_hs_reg(&mut self, inst: u32) -> CpuAction {
        let hs = (inst as u8 & 0b1100000) >> 5;
        let load = inst.bit(20);
        let writeback = inst.bit(21);
        let add = inst.bit(23);
        let pre_index = inst.bit(24);

        let offset = self.get_reg((inst & 0xF) as usize);
        self.ldr_str_common(inst, offset, hs, load, writeback, pre_index, add)
    }

    fn ldr_str_common(
        &mut self,
        inst: u32,
        offset: u32,
        hs: u8,
        load: bool,
        writeback: bool,
        pre_index: bool,
        add: bool,
    ) -> CpuAction {
        let mut ret = CpuAction::AdvancePC;

        let offset = if add {
            offset
        } else {
            (-(offset as i32)) as u32
        };

        let base_reg = inst.bit_range(16..20) as usize;
        let dest_reg = inst.bit_range(12..16) as usize;
        let mut addr = self.get_reg(base_reg);
        if base_reg == REG_PC {
            addr = self.pc_arm() + 8;
        }

        let old_mode = self.cspr.mode();
        if !pre_index && writeback {
            self.change_mode(old_mode, CpuMode::User);
        }

        let effective_addr = (addr as i32).wrapping_add(offset as i32) as Addr;
        addr = if pre_index { effective_addr } else { addr };

        let transfer_type = ArmHalfwordTransferType::from_u8(hs).unwrap();

        if load {
            let data = match transfer_type {
                ArmHalfwordTransferType::SignedByte => self.load_8(addr) as i8 as u32,
                ArmHalfwordTransferType::SignedHalfwords => self.ldr_sign_half(addr),
                ArmHalfwordTransferType::UnsignedHalfwords => self.ldr_half(addr),
            };

            self.set_reg(dest_reg, data);

            // TODO: +1I
            // self.idle_cycle();

            if dest_reg == REG_PC {
                self.reload_pipeline_arm();
                ret = CpuAction::PipelineFlushed;
            }
        } else {
            let value = if dest_reg == REG_PC {
                self.pc_arm() + 12
            } else {
                self.get_reg(dest_reg)
            };

            match transfer_type {
                ArmHalfwordTransferType::UnsignedHalfwords => {
                    self.store_aligned_16(addr, value as u16);
                }
                _ => panic!("invalid HS flags for L=0"),
            };
        }

        if !load || base_reg != dest_reg {
            if !pre_index {
                self.set_reg(base_reg, effective_addr);
            } else if writeback {
                self.set_reg(base_reg, effective_addr);
            }
        }

        ret
    }

    fn data_processing(&mut self, inst: u32) -> CpuAction {
        use AluOpCode::*;

        let op = inst.bit_range(21..25);
        let imm = inst.bit(25);
        let mut set_flags = inst.bit(20);
        let shift_by_reg = inst.bit(4);

        let rn = inst.bit_range(16..20) as usize;
        let rd = inst.bit_range(12..16) as usize;
        let mut op1 = if rn == REG_PC {
            self.pc_arm() + 8
        } else {
            self.get_reg(rn)
        };

        let opcode = AluOpCode::from_u32(op).unwrap();
        let mut carry = self.cspr.c();

        let op2 = if imm {
            let immediate = inst & 0xFF;
            let rotate = 2 * inst.bit_range(8..12);
            self.ror(immediate, rotate, &mut carry, false, true)
        } else {
            let reg = inst & 0xF;

            let shift_by = if shift_by_reg {
                if rn == REG_PC {
                    op1 += 4;
                }
                // TODO: self.idle_cycle();
                let rs = inst.bit_range(8..12) as usize;
                ShiftRegisterBy::ByRegister(rs)
            } else {
                let amount = inst.bit_range(7..11);
                ShiftRegisterBy::ByAmount(amount)
            };

            let shifted_reg = ShiftedRegister {
                reg: reg as usize,
                shift_by,
                bs_op: inst.get_bs_op(),
                added: None,
            };
            self.register_shift(&shifted_reg, &mut carry)
        };

        if rd == REG_PC && set_flags {
            self.transfer_spsr_mode();
            set_flags = false;
        }

        let alu_res = if set_flags {
            let overflow = self.cspr.v();
            let res = match opcode {
                AND | TST => op1 & op2,
                EOR | TEQ => op1 & op2,
                SUB | CMP => todo!(),
                RSB => todo!(),
                ADD | CMN => todo!(),
                ADC => todo!(),
                SBC => todo!(),
                RSC => todo!(),
                ORR => op1 | op2,
                MOV => op2,
                BIC => op1 & !op2,
                MVN => !op2,
            };

            self.alu_update_flags(res, opcode.is_arithmetic(), carry, overflow);
            if opcode.is_settings_flags() {
                None
            } else {
                Some(res)
            }
        } else {
            let c = carry as u32;
            Some(match opcode {
                AND => op1 & op2,
                EOR => op1 ^ op2,
                SUB => op1.wrapping_sub(op2),
                RSB => op2.wrapping_sub(op1),
                ADD => op1.wrapping_add(op2),
                ADC => op1.wrapping_add(op2).wrapping_add(c),
                SBC => op1.wrapping_sub(op2.wrapping_add(1 - c)),
                RSC => op2.wrapping_sub(op1.wrapping_add(1 - c)),
                ORR => op1 | op2,
                MOV => op2,
                BIC => op1 & !op2,
                MVN => !op2,
                _ => unreachable!(),
            })
        };

        let mut ret = CpuAction::AdvancePC;
        if let Some(alu_res) = alu_res {
            self.set_reg(rd, alu_res as u32);
            if rd == REG_PC {
                match self.cspr.state() {
                    CpuState::ARM => self.reload_pipeline_arm(),
                    CpuState::THUMB => self.reload_pipeline_thumb(),
                };
                ret = CpuAction::PipelineFlushed;
            }
        }

        ret
    }

    fn ldm_stm(&mut self, i: u32) -> CpuAction {
        let _load = i.bit(20);
        let _writeback = i.bit(21);
        let _flag_s = i.bit(22);
        let _add = i.bit(23);
        let _pre_index = i.bit(24);

        todo!()
    }

    fn ldr_str(&mut self, i: u32) -> CpuAction {
        let _load = i.bit(20);
        let _writeback = i.bit(21);
        let _byte = i.bit(22);
        let _add = i.bit(23);
        let _pre_index = i.bit(24);
        let _shift = i.bit(25);
        let _bs_op = i.bit_range(5..7) as u8;
        let _shift_by_reg = i.bit(4);

        todo!()
    }

    fn mrs(&mut self, i: u32) -> CpuAction {
        let _spsr_flag = i.bit(22);

        todo!()
    }

    fn transfer_to_status(&mut self, i: u32) -> CpuAction {
        let _imm = i.bit(25);
        let _spsr_flag = i.bit(22);

        todo!()
    }

    fn undefined(&mut self, inst: u32) -> CpuAction {
        panic!(
            "executing undefined arm instruction {:08x} at @{:08x}",
            inst,
            self.pc_arm()
        )
    }

    fn transfer_spsr_mode(&mut self) {
        let spsr = self.spsr;
        if self.cspr.mode() != spsr.mode() {
            self.change_mode(self.cspr.mode(), spsr.mode());
        }
        self.cspr = spsr;
    }
}
