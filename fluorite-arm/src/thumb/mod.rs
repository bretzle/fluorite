use crate::arm::ArmCond;

use super::{
    alu::{AluOpCode, BarrelShiftOpCode},
    Addr, InstructionDecoder,
};
use byteorder::{LittleEndian, ReadBytesExt};
use enum_primitive_derive::*;
use fluorite_common::BitIndex;
use num_traits::FromPrimitive;

pub mod dissasembly;
pub mod exec;

#[derive(Debug, Clone, PartialEq)]
pub struct ThumbInstruction {
    pub fmt: ThumbFormat,
    pub raw: u16,
    pub pc: Addr,
}

impl ThumbInstruction {
    pub fn new(fmt: ThumbFormat, raw: u16, pc: Addr) -> Self {
        Self { fmt, raw, pc }
    }
}

impl InstructionDecoder for ThumbInstruction {
    type IntType = u16;

    fn decode(raw: Self::IntType, addr: Addr) -> Self {
        Self::new(raw.into(), raw, addr)
    }

    fn decode_bytes(bytes: &[u8], addr: Addr) -> Self {
        let mut rdr = std::io::Cursor::new(bytes);
        let raw = rdr.read_u16::<LittleEndian>().unwrap();
        Self::decode(raw, addr)
    }

    fn get_raw(&self) -> Self::IntType {
        self.raw
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ThumbFormat {
    /// 1
    MoveShiftedReg,
    /// 2
    AddSub,
    /// 3
    DataProcessImm,
    /// 4
    AluOps,
    /// 5
    HiRegOpOrBranchExchange,
    /// 6
    LdrPc,
    /// 7
    LdrStrRegOffset,
    /// 8
    LdrStrSHB,
    /// 9
    LdrStrImmOffset,
    /// 10
    LdrStrHalfWord,
    /// 11
    LdrStrSp,
    /// 12
    LoadAddress,
    /// 13
    AddSp,
    /// 14
    PushPop,
    /// 15
    LdmStm,
    /// 16
    BranchConditional,
    /// 17
    Swi,
    /// 18
    Branch,
    /// 19
    BranchLongWithLink,

    /// Not an actual thumb format
    Undefined,
}

impl From<u16> for ThumbFormat {
    fn from(raw: u16) -> Self {
        use ThumbFormat::*;
        if raw & 0xf800 == 0x1800 {
            AddSub
        } else if raw & 0xe000 == 0x0000 {
            MoveShiftedReg
        } else if raw & 0xe000 == 0x2000 {
            DataProcessImm
        } else if raw & 0xfc00 == 0x4000 {
            AluOps
        } else if raw & 0xfc00 == 0x4400 {
            HiRegOpOrBranchExchange
        } else if raw & 0xf800 == 0x4800 {
            LdrPc
        } else if raw & 0xf200 == 0x5000 {
            LdrStrRegOffset
        } else if raw & 0xf200 == 0x5200 {
            LdrStrSHB
        } else if raw & 0xe000 == 0x6000 {
            LdrStrImmOffset
        } else if raw & 0xf000 == 0x8000 {
            LdrStrHalfWord
        } else if raw & 0xf000 == 0x9000 {
            LdrStrSp
        } else if raw & 0xf000 == 0xa000 {
            LoadAddress
        } else if raw & 0xff00 == 0xb000 {
            AddSp
        } else if raw & 0xf600 == 0xb400 {
            PushPop
        } else if raw & 0xf000 == 0xc000 {
            LdmStm
        } else if raw & 0xff00 == 0xdf00 {
            Swi
        } else if raw & 0xf000 == 0xd000 {
            BranchConditional
        } else if raw & 0xf800 == 0xe000 {
            Branch
        } else if raw & 0xf000 == 0xf000 {
            BranchLongWithLink
        } else {
            Undefined
        }
    }
}

pub trait ThumbDecodeHelper {
    fn rs(&self) -> usize;

    fn rb(&self) -> usize;

    fn ro(&self) -> usize;

    fn rn(&self) -> usize;

    fn format1_op(&self) -> BarrelShiftOpCode;

    fn format3_op(&self) -> OpFormat3;

    fn format5_op(&self) -> OpFormat5;

    fn format4_alu_op(&self) -> ThumbAluOps;

    fn offset5(&self) -> u8;

    fn bcond_offset(&self) -> i32;

    fn offset11(&self) -> i32;

    fn word8(&self) -> u16;

    fn is_load(&self) -> bool;

    fn is_subtract(&self) -> bool;

    fn is_immediate_operand(&self) -> bool;

    fn cond(&self) -> ArmCond;

    fn flag(self, bit: usize) -> bool;

    fn register_list(&self) -> u8;

    fn sword7(&self) -> i32;
}

impl ThumbDecodeHelper for u16 {
    #[inline]
    fn rs(&self) -> usize {
        self.bit_range(3..6) as usize
    }

    #[inline]
    /// Note: not true for LdmStm
    fn rb(&self) -> usize {
        self.bit_range(3..6) as usize
    }

    #[inline]
    fn ro(&self) -> usize {
        self.bit_range(6..9) as usize
    }

    #[inline]
    fn rn(&self) -> usize {
        self.bit_range(6..9) as usize
    }

    #[inline]
    fn format1_op(&self) -> BarrelShiftOpCode {
        BarrelShiftOpCode::from_u8(self.bit_range(11..13) as u8).unwrap()
    }

    #[inline]
    fn format3_op(&self) -> OpFormat3 {
        OpFormat3::from_u8(self.bit_range(11..13) as u8).unwrap()
    }

    #[inline]
    fn format5_op(&self) -> OpFormat5 {
        OpFormat5::from_u8(self.bit_range(8..10) as u8).unwrap()
    }

    #[inline]
    fn format4_alu_op(&self) -> ThumbAluOps {
        ThumbAluOps::from_u16(self.bit_range(6..10)).unwrap()
    }

    #[inline]
    fn offset5(&self) -> u8 {
        self.bit_range(6..11) as u8
    }

    #[inline]
    fn bcond_offset(&self) -> i32 {
        ((((*self & 0xff) as u32) << 24) as i32) >> 23
    }

    #[inline]
    fn offset11(&self) -> i32 {
        (*self & 0x7FF) as i32
    }

    #[inline]
    fn word8(&self) -> u16 {
        (*self & 0xff) << 2
    }

    #[inline]
    fn is_load(&self) -> bool {
        self.bit(11)
    }

    #[inline]
    fn is_subtract(&self) -> bool {
        self.bit(9)
    }

    #[inline]
    fn is_immediate_operand(&self) -> bool {
        self.bit(10)
    }

    #[inline]
    fn cond(&self) -> ArmCond {
        ArmCond::from_u8(self.bit_range(8..12) as u8).expect("bad condition")
    }

    #[inline]
    fn flag(self, bit: usize) -> bool {
        self.bit(bit)
    }

    #[inline]
    fn register_list(&self) -> u8 {
        (*self & 0xff) as u8
    }

    #[inline]
    fn sword7(&self) -> i32 {
        let imm7 = *self & 0x7f;
        if self.bit(7) {
            -((imm7 << 2) as i32)
        } else {
            (imm7 << 2) as i32
        }
    }
}

#[derive(Debug, PartialEq, Primitive)]
pub enum OpFormat3 {
    MOV = 0,
    CMP = 1,
    ADD = 2,
    SUB = 3,
}

impl From<OpFormat3> for AluOpCode {
    fn from(op: OpFormat3) -> AluOpCode {
        match op {
            OpFormat3::MOV => AluOpCode::MOV,
            OpFormat3::CMP => AluOpCode::CMP,
            OpFormat3::ADD => AluOpCode::ADD,
            OpFormat3::SUB => AluOpCode::SUB,
        }
    }
}

#[derive(Debug, PartialEq, Primitive)]
pub enum OpFormat5 {
    ADD = 0,
    CMP = 1,
    MOV = 2,
    BX = 3,
}

#[derive(Debug, PartialEq, Primitive)]
pub enum ThumbAluOps {
    AND = 0b0000,
    EOR = 0b0001,
    LSL = 0b0010,
    LSR = 0b0011,
    ASR = 0b0100,
    ADC = 0b0101,
    SBC = 0b0110,
    ROR = 0b0111,
    TST = 0b1000,
    NEG = 0b1001,
    CMP = 0b1010,
    CMN = 0b1011,
    ORR = 0b1100,
    MUL = 0b1101,
    BIC = 0b1110,
    MVN = 0b1111,
}

impl ThumbAluOps {
    pub fn is_setting_flags(&self) -> bool {
        use ThumbAluOps::*;
        matches!(self, TST | CMP | CMN)
    }

    pub fn is_arithmetic(&self) -> bool {
        use ThumbAluOps::*;
        matches!(self, ADC | SBC | NEG | CMP | CMN)
    }
}

impl From<OpFormat5> for AluOpCode {
    fn from(op: OpFormat5) -> AluOpCode {
        match op {
            OpFormat5::ADD => AluOpCode::ADD,
            OpFormat5::CMP => AluOpCode::CMP,
            OpFormat5::MOV => AluOpCode::MOV,
            OpFormat5::BX => panic!("this should not be called if op = BX"),
        }
    }
}
