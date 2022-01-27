use crate::{
    arm::{ArmCond, ArmInstruction},
    memory::{
        MemoryAccess::{self, *},
        MemoryInterface,
    },
    registers::{BankedRegisters, CpuMode, CpuState, StatusRegister},
    thumb::ThumbInstruction,
    Addr, InstructionDecoder,
};
use fluorite_common::{BitIndex, Shared, WeakPointer};
use num_traits::FromPrimitive;

#[derive(Clone)]
pub struct Arm7tdmi<Memory: MemoryInterface> {
    pub(crate) bus: Shared<Memory>,

    // registers
    pub pc: Addr,
    pub(crate) gpr: [u32; 15],
    pub(crate) cspr: StatusRegister,
    pub(crate) spsr: StatusRegister,
    pub(crate) banks: BankedRegisters,

    // pipelining
    pipeline: [u32; 2],
    next_fetch_access: MemoryAccess,

    _options: (),
}

impl<Memory: MemoryInterface> Arm7tdmi<Memory> {
    pub fn new(bus: Shared<Memory>) -> Self {
        Self {
            bus,
            pc: 0,
            gpr: [0; 15],
            cspr: StatusRegister::new().with_mode(CpuMode::System),
            spsr: StatusRegister::new(),
            banks: BankedRegisters::default(),
            pipeline: [0; 2],
            next_fetch_access: MemoryAccess::NonSeq,
            _options: (),
        }
    }

    pub fn weak_ptr(&mut self) -> WeakPointer<Self> {
        WeakPointer::new(self as *mut Self)
    }

    pub fn pc_arm(&self) -> Addr {
        self.pc.wrapping_sub(8)
    }

    pub fn pc_thumb(&self) -> Addr {
        self.pc.wrapping_sub(4)
    }

    pub fn get_next_pc(&self) -> Addr {
        let size = self.word_size() as u32;
        self.pc - 2 * size
    }

    pub fn get_instructionge(&mut self, addr: u32, buffer: &mut String) -> u32 {
        use std::fmt::Write;

        match self.get_cpu_state() {
            CpuState::ARM => {
                let opcode = self.load_32(addr, MemoryAccess::Debugging);
                let decoded = ArmInstruction::decode(opcode, addr);
                write!(buffer, "{}", decoded).unwrap();
                opcode
            }
            CpuState::THUMB => {
                let opcode = self.load_16(addr, MemoryAccess::Debugging);
                let decoded = ThumbInstruction::decode(opcode, addr);
                write!(buffer, "{}", decoded).unwrap();
                opcode as u32
            }
        }
    }

    pub fn get_reg(&self, reg: usize) -> u32 {
        match reg {
            0..=14 => self.gpr[reg],
            15 => self.pc,
            _ => panic!("invalid register {}", reg),
        }
    }

    pub fn set_reg(&mut self, reg: usize, val: u32) {
        match reg {
            0..=14 => self.gpr[reg] = val,
            15 => {
                self.pc = {
                    match self.cspr.state() {
                        CpuState::THUMB => val & !1,
                        CpuState::ARM => val & !3,
                    }
                }
            }
            _ => panic!("invalid register {}", reg),
        }
    }

    pub fn get_registers(&self) -> [u32; 15] {
        self.gpr
    }

    pub fn get_cspr(&self) -> u32 {
        self.cspr.into()
    }

    pub fn word_size(&self) -> usize {
        match self.cspr.state() {
            CpuState::ARM => 4,
            CpuState::THUMB => 2,
        }
    }

    pub fn get_cpu_state(&self) -> CpuState {
        self.cspr.state()
    }

    pub fn step(&mut self) {
        match self.cspr.state() {
            CpuState::ARM => {
                let pc = self.pc & !3;

                let fetched = self.load_32(pc, self.next_fetch_access);
                let inst = self.pipeline[0];
                self.pipeline[0] = self.pipeline[1];
                self.pipeline[1] = fetched;

                let cond = ArmCond::from_u8(inst.bit_range(28..32) as u8).unwrap();
                if cond != ArmCond::AL && !self.check_cond(cond) {
                    self.advance_arm();
                    self.next_fetch_access = MemoryAccess::NonSeq;
                    return;
                }

                match self.execute_arm(inst) {
                    CpuAction::AdvancePC(access) => {
                        self.next_fetch_access = access;
                        self.advance_arm();
                    }
                    CpuAction::PipelineFlushed => {}
                }
            }
            CpuState::THUMB => {
                let pc = self.pc & !1;

                let fetched_now = self.load_16(pc, self.next_fetch_access);
                let insn = self.pipeline[0];
                self.pipeline[0] = self.pipeline[1];
                self.pipeline[1] = fetched_now as u32;
                match self.execute_thumb(insn as u16) {
                    CpuAction::AdvancePC(access) => {
                        self.advance_thumb();
                        self.next_fetch_access = access;
                    }
                    CpuAction::PipelineFlushed => {}
                }
            }
        }
    }

    fn advance_arm(&mut self) {
        self.pc = self.pc.wrapping_add(4)
    }

    fn advance_thumb(&mut self) {
        self.pc = self.pc.wrapping_add(2)
    }

    pub(crate) fn check_cond(&self, cond: ArmCond) -> bool {
        use ArmCond::*;
        match cond {
            EQ => self.cspr.z(),
            NE => !self.cspr.z(),
            HS => self.cspr.c(),
            LO => !self.cspr.c(),
            MI => self.cspr.n(),
            PL => !self.cspr.n(),
            VS => self.cspr.v(),
            VC => !self.cspr.v(),
            HI => self.cspr.c() && !self.cspr.z(),
            LS => !self.cspr.c() || self.cspr.z(),
            GE => self.cspr.n() == self.cspr.v(),
            LT => self.cspr.n() != self.cspr.v(),
            GT => !self.cspr.z() && (self.cspr.n() == self.cspr.v()),
            LE => self.cspr.z() || (self.cspr.n() != self.cspr.v()),
            AL => true,
            Invalid => unreachable!(),
        }
    }

    pub(crate) fn reload_pipeline_arm(&mut self) {
        self.pipeline[0] = self.load_32(self.pc, NonSeq);
        self.advance_arm();
        self.pipeline[1] = self.load_32(self.pc, Seq);
        self.advance_arm();
        self.next_fetch_access = Seq;
    }

    pub(crate) fn reload_pipeline_thumb(&mut self) {
        self.pipeline[0] = self.load_16(self.pc, NonSeq) as u32;
        self.advance_thumb();
        self.pipeline[1] = self.load_16(self.pc, Seq) as u32;
        self.advance_thumb();
        self.next_fetch_access = Seq;
    }

    pub(crate) fn change_mode(&mut self, old: CpuMode, new: CpuMode) {
        let new_index = new.bank_index();
        let old_index = old.bank_index();

        if new_index == old_index {
            return;
        }

        let banks = &mut self.banks;

        banks.spsr_bank[old_index] = self.spsr;
        banks.gpr_banked_r13[old_index] = self.gpr[13];
        banks.gpr_banked_r14[old_index] = self.gpr[14];

        self.spsr = banks.spsr_bank[new_index];
        self.gpr[13] = banks.gpr_banked_r13[new_index];
        self.gpr[14] = banks.gpr_banked_r14[new_index];

        if new == CpuMode::Fiq {
            for r in 0..5 {
                banks.gpr_banked_old_r8_12[r] = self.gpr[r + 8];
                self.gpr[r + 8] = banks.gpr_banked_fiq_r8_12[r];
            }
        } else if old == CpuMode::Fiq {
            for r in 0..5 {
                banks.gpr_banked_fiq_r8_12[r] = self.gpr[r + 8];
                self.gpr[r + 8] = banks.gpr_banked_old_r8_12[r];
            }
        }
        self.cspr.set_mode(new);
    }

    pub fn skip_bios(&mut self) {
        self.banks.gpr_banked_r13[0] = 0x0300_7f00; // USR/SYS
        self.banks.gpr_banked_r13[1] = 0x0300_7f00; // FIQ
        self.banks.gpr_banked_r13[2] = 0x0300_7fa0; // IRQ
        self.banks.gpr_banked_r13[3] = 0x0300_7fe0; // SVC
        self.banks.gpr_banked_r13[4] = 0x0300_7f00; // ABT
        self.banks.gpr_banked_r13[5] = 0x0300_7f00; // UND

        self.pc = 0x0800_0000;
        self.gpr[13] = 0x0300_7F00;
        self.cspr = StatusRegister::from(0x5F);

        // fill pipeline
        self.step();
        self.step();
    }

    pub(crate) fn get_required_multipiler_array_cycles(&self, rs: u32) -> usize {
        if rs & 0xff == rs {
            1
        } else if rs & 0xffff == rs {
            2
        } else if rs & 0xffffff == rs {
            3
        } else {
            4
        }
    }
}

pub enum CpuAction {
    AdvancePC(MemoryAccess),
    PipelineFlushed,
}
