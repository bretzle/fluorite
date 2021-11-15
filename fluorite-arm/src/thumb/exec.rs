#![allow(clippy::too_many_arguments, non_snake_case)]

use super::{OpFormat5, ThumbDecodeHelper};
use crate::{
    alu::BarrelShiftOpCode,
    arm::ArmCond,
    cpu::{Arm7tdmi, CpuAction},
    exception::Exception,
    memory::{
        MemoryAccess::{self, *},
        MemoryInterface,
    },
    thumb::{OpFormat3, ThumbAluOps, ThumbInstruction},
    Addr, InstructionDecoder, REG_LR, REG_PC, REG_SP,
};
use fluorite_common::BitIndex;
use num_traits::FromPrimitive;

impl<Memory: MemoryInterface> Arm7tdmi<Memory> {
    pub(crate) fn execute_thumb(&mut self, inst: u16) -> CpuAction {
        use crate::thumb::ThumbFormat::*;
        let decoded = ThumbInstruction::decode(inst, self.pc_thumb());

		#[cfg(debug_assertions)]
        println!(
            "{:8x}:\t{:08x} \t{}",
            self.pc_thumb(),
            decoded.get_raw(),
            decoded
        );

        let func = match decoded.fmt {
            MoveShiftedReg => Self::thumb_move_shifted_reg,
            AddSub => Self::thumb_add_sub,
            DataProcessImm => Self::thumb_data_process_imm,
            AluOps => Self::thumb_alu_ops,
            HiRegOpOrBranchExchange => Self::thumb_hi_reg_op_or_bx,
            LdrPc => Self::thumb_ldr_pc,
            LdrStrRegOffset => Self::thumb_ldr_str_reg_offset,
            LdrStrSHB => Self::thumb_ldr_str_shb,
            LdrStrImmOffset => Self::thumb_ldr_str_imm_offset,
            LdrStrHalfWord => Self::thumb_ldr_str_halfword,
            LdrStrSp => Self::thumb_ldr_str_sp,
            LoadAddress => Self::thumb_load_address,
            AddSp => Self::thumb_add_sp,
            PushPop => Self::thumb_push_pop,
            LdmStm => Self::thumb_ldm_stm,
            BranchConditional => Self::thumb_branch_with_cond,
            Swi => Self::thumb_swi,
            Branch => Self::thumb_branch,
            BranchLongWithLink => Self::thumb_branch_long_with_link,
            Undefined => todo!(),
        };

        func(self, inst)
    }

    /// Format 1
    /// Execution Time: 1S
    fn thumb_move_shifted_reg(&mut self, inst: u16) -> CpuAction {
        let bs_op = inst.bit_range(11..13) as u8;
        let imm = inst.bit_range(6..11) as u8;

        let rd = (inst & 0b111) as usize;
        let rs = inst.bit_range(3..6) as usize;

        let shift_amount = imm as u32;
        let mut carry = self.cspr.c();
        let bsop = match bs_op {
            0 => BarrelShiftOpCode::LSL,
            1 => BarrelShiftOpCode::LSR,
            2 => BarrelShiftOpCode::ASR,
            3 => BarrelShiftOpCode::ROR,
            _ => unsafe { std::hint::unreachable_unchecked() },
        };
        let op2 = self.barrel_shift_op(bsop, self.gpr[rs], shift_amount, &mut carry, true);
        self.gpr[rd] = op2;
        self.alu_update_flags(op2, false, carry, self.cspr.v());

        CpuAction::AdvancePC(Seq)
    }

    /// Format 2
    /// Execution Time: 1S
    fn thumb_add_sub(&mut self, inst: u16) -> CpuAction {
        let sub = inst.bit(9);
        let imm = inst.bit(10);
        let rn = inst.bit_range(6..9) as usize;

        let rd = (inst & 0b111) as usize;
        let op1 = self.get_reg(inst.rs());
        let op2 = if imm { rn as u32 } else { self.get_reg(rn) };

        let mut carry = self.cspr.c();
        let mut overflow = self.cspr.v();
        let result = if sub {
            self.alu_sub_flags(op1, op2, &mut carry, &mut overflow)
        } else {
            self.alu_add_flags(op1, op2, &mut carry, &mut overflow)
        };
        self.alu_update_flags(result, true, carry, overflow);
        self.set_reg(rd, result as u32);

        CpuAction::AdvancePC(Seq)
    }

    /// Format 3
    /// Execution Time: 1S
    fn thumb_data_process_imm(&mut self, inst: u16) -> CpuAction {
        use OpFormat3::*;

        let op = inst.bit_range(11..13) as u8;
        let rd = inst.bit_range(8..11) as usize;

        let op = OpFormat3::from_u8(op).unwrap();
        let op1 = self.gpr[rd];
        let op2_imm = (inst & 0xff) as u32;
        let mut carry = self.cspr.c();
        let mut overflow = self.cspr.v();
        let result = match op {
            MOV => op2_imm,
            CMP | SUB => self.alu_sub_flags(op1, op2_imm, &mut carry, &mut overflow),
            ADD => self.alu_add_flags(op1, op2_imm, &mut carry, &mut overflow),
        };
        let arithmetic = op == ADD || op == SUB;
        self.alu_update_flags(result, arithmetic, carry, overflow);
        if op != CMP {
            self.gpr[rd] = result as u32;
        }

        CpuAction::AdvancePC(Seq)
    }

    /// Format 4
    /// Execution Time:
    ///     1S      for  AND,EOR,ADC,SBC,TST,NEG,CMP,CMN,ORR,BIC,MVN
    ///     1S+1I   for  LSL,LSR,ASR,ROR
    ///     1S+mI   for  MUL on ARMv4 (m=1..4; depending on MSBs of incoming Rd value)
    fn thumb_alu_ops(&mut self, inst: u16) -> CpuAction {
        let op = inst.bit_range(6..10) as u16;

        let rd = (inst & 0b111) as usize;
        let rs = inst.rs();
        let dst = self.get_reg(rd);
        let src = self.get_reg(rs);

        let mut carry = self.cspr.c();
        let mut overflow = self.cspr.v();

        use ThumbAluOps::*;
        let op = ThumbAluOps::from_u16(op).unwrap();

        macro_rules! shifter_op {
            ($bs_op:expr) => {{
                let result = self.shift_by_register($bs_op, rd, rs, &mut carry);
                self.idle_cycle();
                result
            }};
        }

        let result = match op {
            AND | TST => dst & src,
            EOR => dst ^ src,
            LSL => shifter_op!(BarrelShiftOpCode::LSL),
            LSR => shifter_op!(BarrelShiftOpCode::LSR),
            ASR => shifter_op!(BarrelShiftOpCode::ASR),
            ROR => shifter_op!(BarrelShiftOpCode::ROR),
            ADC => self.alu_adc_flags(dst, src, &mut carry, &mut overflow),
            SBC => self.alu_sbc_flags(dst, src, &mut carry, &mut overflow),
            NEG => self.alu_sub_flags(0, src, &mut carry, &mut overflow),
            CMP => self.alu_sub_flags(dst, src, &mut carry, &mut overflow),
            CMN => self.alu_add_flags(dst, src, &mut carry, &mut overflow),
            ORR => dst | src,
            MUL => {
                let m = self.get_required_multipiler_array_cycles(src);
                for _ in 0..m {
                    self.idle_cycle();
                }
                // TODO - meaningless values?
                carry = false;
                overflow = false;
                dst.wrapping_mul(src)
            }
            BIC => dst & (!src),
            MVN => !src,
        };
        self.alu_update_flags(result, op.is_arithmetic(), carry, overflow);

        if !op.is_setting_flags() {
            self.set_reg(rd, result as u32);
        }

        CpuAction::AdvancePC(Seq)
    }

    /// Format 5
    /// Execution Time:
    ///     1S     for ADD/MOV/CMP
    ///     2S+1N  for ADD/MOV with Rd=R15, and for BX
    fn thumb_hi_reg_op_or_bx(&mut self, inst: u16) -> CpuAction {
        let OP = inst.bit_range(8..10) as u8;
        let FLAG_H1 = inst.bit(7);
        let FLAG_H2 = inst.bit(6);

        let op = OpFormat5::from_u8(OP).unwrap();
        let rd = (inst & 0b111) as usize;
        let rs = inst.rs();
        let dst_reg = if FLAG_H1 { rd + 8 } else { rd };
        let src_reg = if FLAG_H2 { rs + 8 } else { rs };
        let op1 = self.get_reg(dst_reg);
        let op2 = self.get_reg(src_reg);

        let mut result = CpuAction::AdvancePC(Seq);
        match op {
            OpFormat5::BX => {
                return self.branch_exchange(self.get_reg(src_reg));
            }
            OpFormat5::ADD => {
                self.set_reg(dst_reg, op1.wrapping_add(op2));
                if dst_reg == REG_PC {
                    self.reload_pipeline_thumb();
                    result = CpuAction::PipelineFlushed;
                }
            }
            OpFormat5::CMP => {
                let mut carry = self.cspr.c();
                let mut overflow = self.cspr.v();
                let result = self.alu_sub_flags(op1, op2, &mut carry, &mut overflow);
                self.alu_update_flags(result, true, carry, overflow);
            }
            OpFormat5::MOV => {
                self.set_reg(dst_reg, op2 as u32);
                if dst_reg == REG_PC {
                    self.reload_pipeline_thumb();
                    result = CpuAction::PipelineFlushed;
                }
            }
        }

        result
    }

    /// Format 6 load PC-relative (for loading immediates from literal pool)
    /// Execution Time: 1S+1N+1I
    fn thumb_ldr_pc(&mut self, inst: u16) -> CpuAction {
        let rd = inst.bit_range(8..11) as usize;

        let ofs = inst.word8() as Addr;
        let addr = (self.pc & !3) + ofs;

        self.gpr[rd] = self.load_32(addr, NonSeq);

        // +1I
        self.idle_cycle();

        CpuAction::AdvancePC(NonSeq)
    }

    /// Helper function for various ldr/str handler
    /// Execution Time: 1S+1N+1I for LDR, or 2N for STR
    fn do_exec_thumb_ldr_str(
        &mut self,
        inst: u16,
        addr: Addr,
        LOAD: bool,
        BYTE: bool,
    ) -> CpuAction {
        let rd = (inst & 0b111) as usize;
        if LOAD {
            let data = if BYTE {
                self.load_8(addr, NonSeq) as u32
            } else {
                self.ldr_word(addr, NonSeq)
            };

            self.gpr[rd] = data;

            // +1I
            self.idle_cycle();
            CpuAction::AdvancePC(Seq)
        } else {
            let value = self.get_reg(rd);
            if BYTE {
                self.store_8(addr, value as u8, NonSeq);
            } else {
                self.store_aligned_32(addr, value, NonSeq);
            };
            CpuAction::AdvancePC(NonSeq)
        }
    }

    /// Format 7 load/store with register offset
    /// Execution Time: 1S+1N+1I for LDR, or 2N for STR
    fn thumb_ldr_str_reg_offset(&mut self, inst: u16) -> CpuAction {
        let LOAD = inst.bit(11);
        let RO = inst.bit_range(6..9) as usize;
        let BYTE = inst.bit(10);

        let rb = inst.bit_range(3..6) as usize;
        let addr = self.gpr[rb].wrapping_add(self.gpr[RO]);
        self.do_exec_thumb_ldr_str(inst, addr, LOAD, BYTE)
    }

    /// Format 8 load/store sign-extended byte/halfword
    /// Execution Time: 1S+1N+1I for LDR, or 2N for STR
    fn thumb_ldr_str_shb(&mut self, inst: u16) -> CpuAction {
        let RO = inst.bit_range(6..9) as usize;
        let SIGN_EXTEND = inst.bit(10);
        let HALFWORD = inst.bit(11);

        let rb = inst.bit_range(3..6) as usize;
        let rd = (inst & 0b111) as usize;

        let addr = self.gpr[rb].wrapping_add(self.gpr[RO]);
        match (SIGN_EXTEND, HALFWORD) {
            (false, false) =>
            /* strh */
            {
                self.store_aligned_16(addr, self.gpr[rd] as u16, NonSeq);
            }
            (false, true) =>
            /* ldrh */
            {
                self.gpr[rd] = self.ldr_half(addr, NonSeq);
                self.idle_cycle();
            }
            (true, false) =>
            /* ldself */
            {
                let val = self.load_8(addr, NonSeq) as i8 as i32 as u32;
                self.gpr[rd] = val;
                self.idle_cycle();
            }
            (true, true) =>
            /* ldsh */
            {
                let val = self.ldr_sign_half(addr, NonSeq);
                self.gpr[rd] = val;
                self.idle_cycle();
            }
        }

        CpuAction::AdvancePC(NonSeq)
    }

    /// Format 9
    /// Execution Time: 1S+1N+1I for LDR, or 2N for STR
    fn thumb_ldr_str_imm_offset(&mut self, inst: u16) -> CpuAction {
        let LOAD = inst.bit(11);
        let BYTE = inst.bit(12);
        let offset5 = inst.bit_range(6..11) as u8;
        let OFFSET = if BYTE { offset5 } else { (offset5 << 3) >> 1 };

        let rb = inst.bit_range(3..6) as usize;
        let addr = self.gpr[rb].wrapping_add(OFFSET as u32);
        self.do_exec_thumb_ldr_str(inst, addr, LOAD, BYTE)
    }

    /// Format 10
    /// Execution Time: 1S+1N+1I for LDR, or 2N for STR
    fn thumb_ldr_str_halfword(&mut self, inst: u16) -> CpuAction {
        let offset5 = inst.bit_range(6..11) as u8;
        let LOAD = inst.bit(11);
        let OFFSET = (offset5 << 1) as i32;

        let rb = inst.bit_range(3..6) as usize;
        let rd = (inst & 0b111) as usize;
        let base = self.gpr[rb] as i32;
        let addr = base.wrapping_add(OFFSET) as Addr;
        if LOAD {
            let data = self.ldr_half(addr, NonSeq);
            self.idle_cycle();
            self.gpr[rd] = data as u32;
            CpuAction::AdvancePC(Seq)
        } else {
            self.store_aligned_16(addr, self.gpr[rd] as u16, NonSeq);
            CpuAction::AdvancePC(NonSeq)
        }
    }

    /// Format 11 load/store SP-relative
    /// Execution Time: 1S+1N+1I for LDR, or 2N for STR
    fn thumb_ldr_str_sp(&mut self, inst: u16) -> CpuAction {
        let LOAD = inst.bit(11);
        let RD = inst.bit_range(8..11) as usize;

        let addr = self.gpr[REG_SP] + (inst.word8() as Addr);
        if LOAD {
            let data = self.ldr_word(addr, NonSeq);
            self.idle_cycle();
            self.gpr[RD] = data;
            CpuAction::AdvancePC(Seq)
        } else {
            self.store_aligned_32(addr, self.gpr[RD], NonSeq);
            CpuAction::AdvancePC(NonSeq)
        }
    }

    /// Format 12
    /// Execution Time: 1S
    fn thumb_load_address(&mut self, inst: u16) -> CpuAction {
        let SP = inst.bit(11);
        let RD = inst.bit_range(8..11) as usize;

        self.gpr[RD] = if SP {
            self.gpr[REG_SP] + (inst.word8() as Addr)
        } else {
            (self.pc_thumb() & !0b10) + 4 + (inst.word8() as Addr)
        };

        CpuAction::AdvancePC(Seq)
    }

    /// Format 13
    /// Execution Time: 1S
    fn thumb_add_sp(&mut self, inst: u16) -> CpuAction {
        let FLAG_S = inst.bit(7);

        let op1 = self.gpr[REG_SP] as i32;
        let offset = ((inst & 0x7f) << 2) as i32;
        self.gpr[REG_SP] = if FLAG_S {
            op1.wrapping_sub(offset) as u32
        } else {
            op1.wrapping_add(offset) as u32
        };

        CpuAction::AdvancePC(Seq)
    }
    /// Format 14
    /// Execution Time: nS+1N+1I (POP), (n+1)S+2N+1I (POP PC), or (n-1)S+2N (PUSH).
    fn thumb_push_pop(&mut self, inst: u16) -> CpuAction {
        let POP = inst.bit(11);
        let FLAG_R = inst.bit(8);

        macro_rules! push {
            ($r:expr, $access:ident) => {
                self.gpr[REG_SP] -= 4;
                let stack_addr = self.gpr[REG_SP] & !3;
                self.store_32(stack_addr, self.get_reg($r), $access);
                $access = Seq;
            };
        }
        macro_rules! pop {
            ($r:expr) => {
                let val = self.load_32(self.gpr[REG_SP] & !3, Seq);
                self.set_reg($r, val);
                self.gpr[REG_SP] += 4;
            };
            ($r:expr, $access:ident) => {
                let val = self.load_32(self.gpr[REG_SP] & !3, $access);
                $access = Seq;
                self.set_reg($r, val);
                self.gpr[REG_SP] += 4;
            };
        }
        let mut result = CpuAction::AdvancePC(NonSeq);
        let rlist = inst.register_list();
        let mut access = MemoryAccess::NonSeq;
        if POP {
            for r in 0..8 {
                if rlist.bit(r) {
                    pop!(r, access);
                }
            }
            if FLAG_R {
                pop!(REG_PC);
                self.pc = self.pc & !1;
                result = CpuAction::PipelineFlushed;
                self.reload_pipeline_thumb();
            }
            // Idle 1 cycle
            self.idle_cycle();
        } else {
            if FLAG_R {
                push!(REG_LR, access);
            }
            for r in (0..8).rev() {
                if rlist.bit(r) {
                    push!(r, access);
                }
            }
        }

        result
    }

    /// Format 15
    /// Execution Time: nS+1N+1I for LDM, or (n-1)S+2N for STM.
    fn thumb_ldm_stm(&mut self, inst: u16) -> CpuAction {
        let LOAD = inst.bit(11);
        let RB = inst.bit_range(8..11) as usize;

        let mut result = CpuAction::AdvancePC(NonSeq);

        let align_preserve = self.gpr[RB] & 3;
        let mut addr = self.gpr[RB] & !3;
        let rlist = inst.register_list();
        // let mut first = true;
        if rlist != 0 {
            if LOAD {
                let mut access = NonSeq;
                for r in 0..8 {
                    if rlist.bit(r) {
                        let val = self.load_32(addr, access);
                        access = Seq;
                        addr += 4;
                        self.set_reg(r, val);
                    }
                }
                self.idle_cycle();
                if !rlist.bit(RB) {
                    self.gpr[RB] = addr + align_preserve;
                }
            } else {
                let mut first = true;
                let mut access = NonSeq;
                for r in 0..8 {
                    if rlist.bit(r) {
                        let v = if r != RB {
                            self.gpr[r]
                        } else {
                            if first {
                                addr
                            } else {
                                addr + (rlist.count_ones() - 1) * 4
                            }
                        };
                        if first {
                            first = false;
                        }
                        self.store_32(addr, v, access);
                        access = Seq;
                        addr += 4;
                    }
                    self.gpr[RB] = addr + align_preserve;
                }
            }
        } else {
            // From gbatek.htm: Empty Rlist: R15 loaded/stored (ARMv4 only), and Rb=Rb+40h (ARMv4-v5).
            if LOAD {
                let val = self.load_32(addr, NonSeq);
                self.pc = val & !1;
                result = CpuAction::PipelineFlushed;
                self.reload_pipeline_thumb();
            } else {
                self.store_32(addr, self.pc + 2, NonSeq);
            }
            addr += 0x40;
            self.gpr[RB] = addr + align_preserve;
        }

        result
    }

    /// Format 16
    /// Execution Time:
    ///     2S+1N   if condition true (jump executed)
    ///     1S      if condition false
    fn thumb_branch_with_cond(&mut self, inst: u16) -> CpuAction {
        let COND = inst.bit_range(8..12) as u8;

        let cond = ArmCond::from_u8(COND).expect("bad cond");
        if !self.check_cond(cond) {
            CpuAction::AdvancePC(Seq)
        } else {
            let offset = inst.bcond_offset();
            self.pc = (self.pc as i32).wrapping_add(offset) as u32;
            self.reload_pipeline_thumb();
            CpuAction::PipelineFlushed
        }
    }

    /// Format 17
    /// Execution Time: 2S+1N
    fn thumb_swi(&mut self, _inst: u16) -> CpuAction {
        self.exception(Exception::SoftwareInterrupt, self.pc - 2); // implies pipeline reload
        CpuAction::PipelineFlushed
    }

    /// Format 18
    /// Execution Time: 2S+1N
    fn thumb_branch(&mut self, inst: u16) -> CpuAction {
        let offset = ((inst.offset11() << 21) >> 20) as i32;
        self.pc = (self.pc as i32).wrapping_add(offset) as u32;
        self.reload_pipeline_thumb(); // 2S + 1N
        CpuAction::PipelineFlushed
    }

    /// Format 19
    /// Execution Time: 3S+1N (first opcode 1S, second opcode 2S+1N).
    fn thumb_branch_long_with_link(&mut self, inst: u16) -> CpuAction {
        let FLAG_LOW_OFFSET = inst.bit(11);

        let mut off = inst.offset11();
        if FLAG_LOW_OFFSET {
            off = off << 1;
            let next_pc = (self.pc - 2) | 1;
            self.pc = ((self.gpr[REG_LR] & !1) as i32).wrapping_add(off) as u32;
            self.gpr[REG_LR] = next_pc;
            self.reload_pipeline_thumb(); // implies 2S + 1N
            CpuAction::PipelineFlushed
        } else {
            off = (off << 21) >> 9;
            self.gpr[REG_LR] = (self.pc as i32).wrapping_add(off) as u32;
            CpuAction::AdvancePC(Seq) // 1S
        }
    }

    pub fn undefined(&mut self, inst: u16) -> CpuAction {
        panic!(
            "executing undefind thumb instruction {:04x} at @{:08x}",
            inst,
            self.pc_thumb()
        )
    }
}
