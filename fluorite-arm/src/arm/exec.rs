#![allow(
    clippy::too_many_arguments,
    clippy::assign_op_pattern,
    clippy::identity_op,
    non_snake_case
)]

use super::{ArmDecodeHelper, ArmHalfwordTransferType};
use crate::{
    alu::{AluOpCode, ShiftRegisterBy, ShiftedRegister},
    arm::ArmInstruction,
    cpu::{Arm7tdmi, CpuAction},
    memory::{MemoryAccess::*, MemoryInterface},
    registers::{CpuMode, CpuState, StatusRegister},
    Addr, InstructionDecoder, REG_LR, REG_PC,
};
use fluorite_common::BitIndex;
use num_traits::FromPrimitive;

impl<Memory: MemoryInterface> Arm7tdmi<Memory> {
    pub(crate) fn execute_arm(&mut self, inst: u32) -> CpuAction {
        use crate::arm::ArmFormat::*;
        let decoded = ArmInstruction::decode(inst, self.pc_arm());

        // println!(
        //     "{:8x}:\t{:08x} \t{}",
        //     self.pc_arm(),
        //     decoded.get_raw(),
        //     decoded
        // );

        let func = match decoded.fmt {
            BranchExchange => Self::arm_bx,
            BranchLink => Self::arm_b_bl,
            SoftwareInterrupt => Self::arm_swi,
            Multiply => Self::arm_mul_mla,
            MultiplyLong => Self::arm_mull_mlal,
            SingleDataTransfer => Self::arm_ldr_str,
            HalfwordDataTransferRegOffset => Self::arm_ldr_str_hs_reg,
            HalfwordDataTransferImmediateOffset => Self::arm_ldr_str_hs_imm,
            DataProcessing => Self::arm_data_processing,
            BlockDataTransfer => Self::arm_ldm_stm,
            SingleDataSwap => Self::arm_swp,
            MoveFromStatus => Self::arm_mrs,
            MoveToStatus => Self::arm_transfer_to_status,
            MoveToFlags => unreachable!(), // what is this???
            Undefined => Self::arm_undefined,
        };

        func(self, inst)
    }

    // fn ldr_str_hs_imm(&mut self, inst: u32) -> CpuAction {
    //     let hs = (inst as u8 & 0b1100000) >> 5;
    //     let load = inst.bit(20);
    //     let writeback = inst.bit(21);
    //     let pre_index = inst.bit(24);
    //     let add = inst.bit(23);

    //     let offset = (inst.bit_range(8..12) << 4) + inst.bit_range(0..4);
    //     self.ldr_str_common(inst, offset, hs, load, writeback, pre_index, add)
    // }

    // fn ldr_str_hs_reg(&mut self, inst: u32) -> CpuAction {
    //     let hs = (inst as u8 & 0b1100000) >> 5;
    //     let load = inst.bit(20);
    //     let writeback = inst.bit(21);
    //     let add = inst.bit(23);
    //     let pre_index = inst.bit(24);

    //     let offset = self.get_reg((inst & 0xF) as usize);
    //     self.ldr_str_common(inst, offset, hs, load, writeback, pre_index, add)
    // }

    fn arm_undefined(&mut self, insn: u32) -> CpuAction {
        panic!(
            "executing undefined arm instruction {:08x} at @{:08x}",
            insn,
            self.pc_arm()
        )
    }

    /// Branch and Branch with Link (B, BL)
    /// Execution Time: 2S + 1N
    fn arm_b_bl(&mut self, inst: u32) -> CpuAction {
        let link = inst.bit(24);

        if link {
            self.set_reg(REG_LR, (self.pc_arm() + (self.word_size() as u32)) & !0b1);
        }
        self.pc = (self.pc as i32).wrapping_add(inst.branch_offset()) as u32 & !1;

        self.reload_pipeline_arm(); // Implies 2S + 1N
        CpuAction::PipelineFlushed
    }

    pub(crate) fn branch_exchange(&mut self, mut addr: Addr) -> CpuAction {
        if addr.bit(0) {
            addr = addr & !0x1;
            self.cspr.set_state(CpuState::THUMB);
            self.pc = addr;
            self.reload_pipeline_thumb();
            // println!("[CPU] State: Arm -> Thumb")
        } else {
            addr = addr & !0x3;
            self.cspr.set_state(CpuState::ARM);
            self.pc = addr;
            self.reload_pipeline_arm();
            // println!("[CPU] State: Thumb -> Arm")
        }
        CpuAction::PipelineFlushed
    }

    /// Branch and Exchange (BX)
    /// Cycles 2S+1N
    fn arm_bx(&mut self, insn: u32) -> CpuAction {
        self.branch_exchange(self.get_reg(insn.bit_range(0..4) as usize))
    }

    /// Move from status register
    /// 1S
    fn arm_mrs(&mut self, insn: u32) -> CpuAction {
        let SPSR_FLAG = insn.bit(22);

        let rd = insn.bit_range(12..16) as usize;
        let result = if SPSR_FLAG {
            self.spsr.into()
        } else {
            self.cspr.into()
        };
        self.set_reg(rd, result);

        CpuAction::AdvancePC(Seq)
    }

    /// Move to status register
    /// 1S
    fn arm_transfer_to_status(&mut self, inst: u32) -> CpuAction {
        let imm = inst.bit(25);
        let spsr_flag = inst.bit(22);

        let value = if imm {
            let immediate = inst & 0xFF;
            let rotate = 2 * inst.bit_range(8..12);
            let mut carry = self.cspr.c();
            let v = self.ror(immediate, rotate, &mut carry, false, true);
            self.cspr.set_c(carry);
            v
        } else {
            self.get_reg((inst & 0xF) as usize)
        };

        let f = inst.bit(19);
        let s = inst.bit(18);
        let x = inst.bit(17);
        let c = inst.bit(16);

        let mut mask = 0;
        if f {
            mask |= 0xff << 24;
        }
        if s {
            mask |= 0xff << 16;
        }
        if x {
            mask |= 0xff << 8;
        }
        if c {
            mask |= 0xff << 0;
        }

        match self.cspr.mode() {
            CpuMode::User => {
                if spsr_flag {
                    panic!("User mode cant access SPSR")
                }
                self.cspr.set_flag_bits(value)
            }
            _ => {
                if spsr_flag {
                    self.spsr = StatusRegister::from(value);
                } else {
                    let old_mode = self.cspr.mode();
                    let new_psr =
                        StatusRegister::from((u32::from(self.cspr) & !mask) | (value & mask));
                    let new_mode = new_psr.mode();
                    if old_mode != new_mode {
                        self.change_mode(old_mode, new_mode);
                    }
                    self.cspr = new_psr;
                }
            }
        }

        CpuAction::AdvancePC(Seq)
    }

    fn transfer_spsr_mode(&mut self) {
        let spsr = self.spsr;
        if self.cspr.mode() != spsr.mode() {
            self.change_mode(self.cspr.mode(), spsr.mode());
        }
        self.cspr = spsr;
    }

    /// Logical/Arithmetic ALU operations
    ///
    /// Cycles: 1S+x+y (from GBATEK)
    ///         Add x=1I cycles if Op2 shifted-by-register. Add y=1S+1N cycles if Rd=R15.
    fn arm_data_processing(&mut self, inst: u32) -> CpuAction {
        let op = inst.bit_range(21..25) as u8;
        let imm = inst.bit(25);
        let mut set_flags = inst.bit(20);
        let shift_by_reg = inst.bit(4);

        use AluOpCode::*;
        let rn = inst.bit_range(16..20) as usize;
        let rd = inst.bit_range(12..16) as usize;
        let mut op1 = if rn == REG_PC {
            self.pc_arm() + 8
        } else {
            self.get_reg(rn)
        };
        let opcode =
            AluOpCode::from_u8(op).unwrap_or_else(|| unsafe { std::hint::unreachable_unchecked() });

        let mut carry = self.cspr.c();
        let op2 = if imm {
            let immediate = inst & 0xff;
            let rotate = 2 * inst.bit_range(8..12);
            // TODO refactor out
            self.ror(immediate, rotate, &mut carry, false, true)
        } else {
            let reg = inst & 0xf;

            let shift_by = if shift_by_reg {
                if rn == REG_PC {
                    op1 += 4;
                }
                self.idle_cycle();
                let rs = inst.bit_range(8..12) as usize;
                ShiftRegisterBy::ByRegister(rs)
            } else {
                let amount = inst.bit_range(7..12) as u32;
                ShiftRegisterBy::ByAmount(amount)
            };

            let shifted_reg = ShiftedRegister {
                reg: reg as usize,
                bs_op: inst.get_bs_op(),
                shift_by,
                added: None,
            };
            self.register_shift(&shifted_reg, &mut carry)
        };

        if rd == REG_PC && set_flags {
            self.transfer_spsr_mode();
            set_flags = false;
        }

        let alu_res = if set_flags {
            let mut overflow = self.cspr.v();
            let result = match opcode {
                AND | TST => op1 & op2,
                EOR | TEQ => op1 ^ op2,
                SUB | CMP => self.alu_sub_flags(op1, op2, &mut carry, &mut overflow),
                RSB => self.alu_sub_flags(op2, op1, &mut carry, &mut overflow),
                ADD | CMN => self.alu_add_flags(op1, op2, &mut carry, &mut overflow),
                ADC => self.alu_adc_flags(op1, op2, &mut carry, &mut overflow),
                SBC => self.alu_sbc_flags(op1, op2, &mut carry, &mut overflow),
                RSC => self.alu_sbc_flags(op2, op1, &mut carry, &mut overflow),
                ORR => op1 | op2,
                MOV => op2,
                BIC => op1 & (!op2),
                MVN => !op2,
            };

            self.alu_update_flags(result, opcode.is_arithmetic(), carry, overflow);

            if opcode.is_settings_flags() {
                None
            } else {
                Some(result)
            }
        } else {
            let c = carry as u32;
            Some(match opcode {
                AND | TST => op1 & op2,
                EOR | TEQ => op1 ^ op2,
                SUB => op1.wrapping_sub(op2),
                RSB => op2.wrapping_sub(op1),
                ADD => op1.wrapping_add(op2),
                ADC => op1.wrapping_add(op2).wrapping_add(c),
                SBC => op1.wrapping_sub(op2.wrapping_add(1 - c)),
                RSC => op2.wrapping_sub(op1.wrapping_add(1 - c)),
                ORR => op1 | op2,
                MOV => op2,
                BIC => op1 & (!op2),
                MVN => !op2,
                _ => panic!("DataProcessing should be a PSR transfer: ({})", opcode),
            })
        };

        let mut result = CpuAction::AdvancePC(Seq);
        if let Some(alu_res) = alu_res {
            self.set_reg(rd, alu_res as u32);
            if rd == REG_PC {
                // T bit might have changed
                match self.cspr.state() {
                    CpuState::ARM => self.reload_pipeline_arm(),
                    CpuState::THUMB => self.reload_pipeline_thumb(),
                };
                result = CpuAction::PipelineFlushed;
            }
        }

        result
    }

    /// Memory Load/Store
    /// Instruction                     |  Cycles       | Flags | Expl.
    /// ------------------------------------------------------------------------------
    /// LDR{cond}{B}{T} Rd,<Address>    | 1S+1N+1I+y    | ----  |  Rd=[Rn+/-<offset>]
    /// STR{cond}{B}{T} Rd,<Address>    | 2N            | ----  |  [Rn+/-<offset>]=Rd
    /// ------------------------------------------------------------------------------
    /// For LDR, add y=1S+1N if Rd=R15.
    fn arm_ldr_str(&mut self, inst: u32) -> CpuAction {
        let load = inst.bit(20);
        let writeback = inst.bit(21);
        let byte = inst.bit(22);
        let add = inst.bit(23);
        let pre_index = inst.bit(24);
        let shift = inst.bit(25);
        let bs_op = inst.bit_range(5..7) as u8;
        let shift_by_reg = inst.bit(4);

        let mut result = CpuAction::AdvancePC(NonSeq);

        let base_reg = inst.bit_range(16..20) as usize;
        let dest_reg = inst.bit_range(12..16) as usize;
        let mut addr = self.get_reg(base_reg);
        if base_reg == REG_PC {
            addr = self.pc_arm() + 8; // prefetching
        }
        let mut offset = inst.bit_range(0..12);
        if shift {
            let mut carry = self.cspr.c();
            let rm = offset & 0xf;
            offset =
                self.register_shift_const(offset, rm as usize, &mut carry, bs_op, shift_by_reg);
        }
        let offset = if add {
            offset as u32
        } else {
            (-(offset as i32)) as u32
        };
        let effective_addr = (addr as i32).wrapping_add(offset as i32) as Addr;

        // TODO - confirm this
        let old_mode = self.cspr.mode();
        if !pre_index && writeback {
            self.change_mode(old_mode, CpuMode::User);
        }

        addr = if pre_index { effective_addr } else { addr };

        if load {
            let data = if byte {
                self.load_8(addr, NonSeq) as u32
            } else {
                self.ldr_word(addr, NonSeq)
            };

            self.set_reg(dest_reg, data);

            // +1I
            self.idle_cycle();

            if dest_reg == REG_PC {
                self.reload_pipeline_arm();
                result = CpuAction::PipelineFlushed;
            }
        } else {
            let value = if dest_reg == REG_PC {
                self.pc_arm() + 12
            } else {
                self.get_reg(dest_reg)
            };
            if byte {
                self.store_8(addr, value as u8, NonSeq);
            } else {
                self.store_aligned_32(addr & !0x3, value, NonSeq);
            };
        }

        if (!load || base_reg != dest_reg) && (!pre_index || writeback) {
            self.set_reg(base_reg, effective_addr);
            self.set_reg(base_reg, effective_addr);
        }

        if !pre_index && writeback {
            self.change_mode(self.cspr.mode(), old_mode);
        }

        result
    }

    fn arm_ldr_str_hs_reg(&mut self, inst: u32) -> CpuAction {
        let hs = (inst as u8 & 0b1100000) >> 5;
        let load = inst.bit(20);
        let writeback = inst.bit(21);
        let add = inst.bit(23);
        let pre_index = inst.bit(24);

        let offset = self.get_reg((inst & 0xf) as usize);
        self.ldr_str_hs_common(inst, offset, hs, load, writeback, pre_index, add)
    }

    fn arm_ldr_str_hs_imm(&mut self, inst: u32) -> CpuAction {
        let hs = (inst as u8 & 0b1100000) >> 5;
        let load = inst.bit(20);
        let writeback = inst.bit(21);
        let pre_index = inst.bit(24);
        let add = inst.bit(23);

        let offset8 = (inst.bit_range(8..12) << 4) + inst.bit_range(0..4);
        self.ldr_str_hs_common(inst, offset8, hs, load, writeback, pre_index, add)
    }

    #[inline(always)]
    fn ldr_str_hs_common(
        &mut self,
        insn: u32,
        offset: u32,
        hs: u8,
        load: bool,
        writeback: bool,
        pre_index: bool,
        add: bool,
    ) -> CpuAction {
        let mut result = CpuAction::AdvancePC(NonSeq);

        let offset = if add {
            offset
        } else {
            (-(offset as i32)) as u32
        };
        let base_reg = insn.bit_range(16..20) as usize;
        let dest_reg = insn.bit_range(12..16) as usize;
        let mut addr = self.get_reg(base_reg);
        if base_reg == REG_PC {
            addr = self.pc_arm() + 8; // prefetching
        }

        // TODO - confirm this
        let old_mode = self.cspr.mode();
        if !pre_index && writeback {
            self.change_mode(old_mode, CpuMode::User);
        }

        let effective_addr = (addr as i32).wrapping_add(offset as i32) as Addr;
        addr = if pre_index { effective_addr } else { addr };

        let transfer_type = ArmHalfwordTransferType::from_u8(hs).unwrap();

        if load {
            let data = match transfer_type {
                ArmHalfwordTransferType::SignedByte => self.load_8(addr, NonSeq) as u8 as i8 as u32,
                ArmHalfwordTransferType::SignedHalfwords => self.ldr_sign_half(addr, NonSeq),
                ArmHalfwordTransferType::UnsignedHalfwords => self.ldr_half(addr, NonSeq),
            };

            self.set_reg(dest_reg, data);

            // +1I
            self.idle_cycle();

            if dest_reg == REG_PC {
                self.reload_pipeline_arm();
                result = CpuAction::PipelineFlushed;
            }
        } else {
            let value = if dest_reg == REG_PC {
                self.pc_arm() + 12
            } else {
                self.get_reg(dest_reg)
            };

            match transfer_type {
                ArmHalfwordTransferType::UnsignedHalfwords => {
                    self.store_aligned_16(addr, value as u16, NonSeq);
                }
                _ => panic!("invalid HS flags for L=0"),
            };
        }

        if (!load || base_reg != dest_reg) && (!pre_index || writeback) {
            self.set_reg(base_reg, effective_addr);
        }

        result
    }

    fn arm_ldm_stm(&mut self, inst: u32) -> CpuAction {
        let load = inst.bit(20);
        let writeback = inst.bit(21);
        let flag_s = inst.bit(22);
        let add = inst.bit(23);
        let pre_index = inst.bit(24);

        let mut result = CpuAction::AdvancePC(NonSeq);

        let mut full = pre_index;
        let ascending = add;
        let mut writeback = writeback;
        let base_reg = inst.bit_range(16..20) as usize;
        let mut base_addr = self.get_reg(base_reg);

        let rlist = inst.register_list();

        if flag_s {
            match self.cspr.mode() {
                CpuMode::User | CpuMode::System => {
                    panic!("LDM/STM with S bit in unprivileged mode")
                }
                _ => {}
            };
        }

        let user_bank_transfer = if flag_s {
            if load {
                !rlist.bit(REG_PC)
            } else {
                true
            }
        } else {
            false
        };

        let old_mode = self.cspr.mode();
        if user_bank_transfer {
            self.change_mode(old_mode, CpuMode::User);
        }

        let psr_transfer = flag_s & load & rlist.bit(REG_PC);

        let rlist_count = rlist.count_ones();

        let old_base = base_addr;

        if rlist != 0 && !ascending {
            base_addr = base_addr.wrapping_sub(rlist_count * 4);
            if writeback {
                self.set_reg(base_reg, base_addr);
                writeback = false;
            }
            full = !full;
        }

        let mut addr = base_addr;

        if rlist != 0 {
            if load {
                let mut access = NonSeq;
                for r in 0..16 {
                    if rlist.bit(r) {
                        if r == base_reg {
                            writeback = false;
                        }
                        if full {
                            addr = addr.wrapping_add(4);
                        }
                        let val = self.load_32(addr, access);
                        access = Seq;
                        self.set_reg(r, val);
                        if r == REG_PC {
                            if psr_transfer {
                                self.transfer_spsr_mode();
                            }
                            self.reload_pipeline_arm();
                            result = CpuAction::PipelineFlushed;
                        }
                        if !full {
                            addr = addr.wrapping_add(4);
                        }
                    }
                }
                self.idle_cycle();
            } else {
                let mut first = true;
                let mut access = NonSeq;
                for r in 0..16 {
                    if rlist.bit(r) {
                        let val = if r != base_reg {
                            if r == REG_PC {
                                self.pc_arm() + 12
                            } else {
                                self.get_reg(r)
                            }
                        } else if first {
                            old_base
                        } else {
                            let x = rlist_count * 4;
                            if ascending {
                                old_base + x
                            } else {
                                old_base - x
                            }
                        };

                        if full {
                            addr = addr.wrapping_add(4);
                        }

                        first = false;

                        self.store_aligned_32(addr, val, access);
                        access = Seq;
                        if !full {
                            addr = addr.wrapping_add(4);
                        }
                    }
                }
            }
        } else {
            if load {
                let val = self.ldr_word(addr, NonSeq);
                self.set_reg(REG_PC, val & !3);
                self.reload_pipeline_arm();
                result = CpuAction::PipelineFlushed;
            } else {
                // block data store with empty rlist
                let addr = match (ascending, full) {
                    (false, false) => addr.wrapping_sub(0x3c),
                    (false, true) => addr.wrapping_sub(0x40),
                    (true, false) => addr,
                    (true, true) => addr.wrapping_add(4),
                };
                self.store_aligned_32(addr, self.pc + 4, NonSeq);
            }
            addr = if ascending {
                addr.wrapping_add(0x40)
            } else {
                addr.wrapping_sub(0x40)
            };
        }

        if user_bank_transfer {
            self.change_mode(self.cspr.mode(), old_mode);
        }

        if writeback {
            self.set_reg(base_reg, addr as u32);
        }

        result
    }

    /// Multiply and Multiply-Accumulate (MUL, MLA)
    /// Execution Time: 1S+mI for MUL, and 1S+(m+1)I for MLA.
    fn arm_mul_mla(&mut self, inst: u32) -> CpuAction {
        let UPDATE_FLAGS = inst.bit(20);
        let ACCUMULATE = inst.bit(21);

        let rd = inst.bit_range(16..20) as usize;
        let rn = inst.bit_range(12..16) as usize;
        let rs = inst.bit_range(8..12) as usize;
        let rm = inst.bit_range(0..4) as usize;

        // // check validity
        // assert!(!(REG_PC == rd || REG_PC == rn || REG_PC == rs || REG_PC == rm));
        // assert!(rd != rm);

        let op1 = self.get_reg(rm);
        let op2 = self.get_reg(rs);
        let mut result = op1.wrapping_mul(op2);

        if ACCUMULATE {
            result = result.wrapping_add(self.get_reg(rn));
            self.idle_cycle();
        }

        self.set_reg(rd, result);

        let m = self.get_required_multipiler_array_cycles(op2);
        for _ in 0..m {
            self.idle_cycle();
        }

        if UPDATE_FLAGS {
            self.cspr.set_n((result as i32) < 0);
            self.cspr.set_z(result == 0);
            self.cspr.set_c(false);
            self.cspr.set_v(false);
        }

        CpuAction::AdvancePC(Seq)
    }

    /// Multiply Long and Multiply-Accumulate Long (MULL, MLAL)
    /// Execution Time: 1S+(m+1)I for MULL, and 1S+(m+2)I for MLAL
    fn arm_mull_mlal(&mut self, inst: u32) -> CpuAction {
        let _update_flags = inst.bit(20);
        let _accumulate = inst.bit(21);
        let _u_flag = inst.bit(22);

        todo!()
    }

    /// ARM Opcodes: Memory: Single Data Swap (SWP)
    /// Execution Time: 1S+2N+1I. That is, 2N data cycles, 1S code cycle, plus 1I.
    fn arm_swp(&mut self, inst: u32) -> CpuAction {
        let _byte = inst.bit(22);

        todo!()
    }

    /// ARM Software Interrupt
    /// Execution Time: 2S+1N
    fn arm_swi(&mut self, inst: u32) -> CpuAction {
        self.software_interrupt(self.pc - 4, inst.swi_comment()); // Implies 2S + 1N
        CpuAction::PipelineFlushed
    }
}
