use crate::{alu::*, Addr, InstructionDecoder};
use byteorder::{LittleEndian, ReadBytesExt};
use enum_primitive_derive::*;
use fluorite_common::BitIndex;
use num_traits::FromPrimitive;

mod disassembly;
mod exec;

#[derive(Debug, Clone, PartialEq)]
pub struct ArmInstruction {
    pub fmt: ArmFormat,
    pub raw: u32,
    pub pc: Addr,
}

impl ArmInstruction {
    pub fn new(raw: u32, pc: Addr, fmt: ArmFormat) -> Self {
        Self { fmt, raw, pc }
    }
}

impl InstructionDecoder for ArmInstruction {
    type IntType = u32;

    fn decode(raw: Self::IntType, addr: Addr) -> Self {
        let fmt = ArmFormat::from(raw);

        Self { fmt, raw, pc: addr }
    }

    fn decode_bytes(bytes: &[u8], addr: Addr) -> Self {
        let mut rdr = std::io::Cursor::new(bytes);
        let raw = rdr.read_u32::<LittleEndian>().unwrap();
        Self::decode(raw, addr)
    }

    fn get_raw(&self) -> Self::IntType {
        self.raw
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum ArmFormat {
    BranchExchange = 0,
    BranchLink,
    SoftwareInterrupt,
    Multiply,
    MultiplyLong,
    SingleDataTransfer,
    HalfwordDataTransferRegOffset,
    HalfwordDataTransferImmediateOffset,
    DataProcessing,
    BlockDataTransfer,
    SingleDataSwap,
    /// Transfer PSR contents to a register
    MoveFromStatus,
    /// Transfer register contents to PSR
    MoveToStatus,
    /// Tanssfer immediate/register to PSR flags only
    MoveToFlags,

    Undefined,
}

impl From<u32> for ArmFormat {
    fn from(raw: u32) -> ArmFormat {
        use ArmFormat::*;
        if (0x0FFF_FFF0 & raw) == 0x012F_FF10 {
            BranchExchange
        } else if (0x0E00_0000 & raw) == 0x0A00_0000 {
            BranchLink
        } else if (0x0FB0_0FF0 & raw) == 0x0100_0090 {
            SingleDataSwap
        } else if (0x0FC0_00F0 & raw) == 0x0000_0090 {
            Multiply
        } else if (0x0F80_00F0 & raw) == 0x0080_0090 {
            MultiplyLong
        } else if (0x0FBF_0FFF & raw) == 0x010F_0000 {
            MoveFromStatus
        } else if (0x0FBF_FFF0 & raw) == 0x0129_F000 {
            MoveToStatus
        } else if (0x0DBF_F000 & raw) == 0x0128_F000 {
            MoveToFlags
        } else if (0x0C00_0000 & raw) == 0x0400_0000 {
            SingleDataTransfer
        } else if (0x0E40_0F90 & raw) == 0x0000_0090 {
            HalfwordDataTransferRegOffset
        } else if (0x0E40_0090 & raw) == 0x0040_0090 {
            HalfwordDataTransferImmediateOffset
        } else if (0x0E00_0000 & raw) == 0x0800_0000 {
            BlockDataTransfer
        } else if (0x0F00_0000 & raw) == 0x0F00_0000 {
            SoftwareInterrupt
        } else if (0x0C00_0000 & raw) == 0x0000_0000 {
            DataProcessing
        } else {
            Undefined
        }
    }
}

pub trait ArmDecodeHelper {
    fn cond(&self) -> ArmCond;

    fn rm(&self) -> usize;

    fn rs(&self) -> usize;

    fn rd_lo(&self) -> usize;

    fn rd_hi(&self) -> usize;

    fn opcode(&self) -> AluOpCode;

    fn branch_offset(&self) -> i32;

    fn load_flag(&self) -> bool;

    fn set_cond_flag(&self) -> bool;

    fn write_back_flag(&self) -> bool;

    fn accumulate_flag(&self) -> bool;

    fn u_flag(&self) -> bool;

    fn halfword_data_transfer_type(&self) -> ArmHalfwordTransferType;

    fn transfer_size(&self) -> usize;

    fn psr_and_force_user_flag(&self) -> bool;

    fn spsr_flag(&self) -> bool;

    fn add_offset_flag(&self) -> bool;

    fn pre_index_flag(&self) -> bool;

    fn link_flag(&self) -> bool;

    /// gets offset used by ldr/str instructions
    fn ldr_str_offset(&self) -> BarrelShifterValue;

    fn get_bs_op(&self) -> BarrelShiftOpCode;

    fn get_shift_reg_by(&self) -> ShiftRegisterBy;

    fn ldr_str_hs_imm_offset(&self) -> BarrelShifterValue;

    fn ldr_str_hs_reg_offset(&self) -> BarrelShifterValue;

    fn operand2(&self) -> BarrelShifterValue;

    fn register_list(&self) -> u16;

    fn swi_comment(&self) -> u32;
}

impl ArmDecodeHelper for u32 {
    #[inline(always)]
    fn cond(&self) -> ArmCond {
        ArmCond::from_u32(self.bit_range(28..32)).unwrap()
    }

    #[inline(always)]
    fn rm(&self) -> usize {
        self.bit_range(0..4) as usize
    }

    #[inline(always)]
    fn rs(&self) -> usize {
        self.bit_range(8..12) as usize
    }

    #[inline(always)]
    fn rd_lo(&self) -> usize {
        self.bit_range(12..16) as usize
    }

    #[inline(always)]
    fn rd_hi(&self) -> usize {
        self.bit_range(16..20) as usize
    }

    #[inline(always)]
    fn opcode(&self) -> AluOpCode {
        use std::hint::unreachable_unchecked;

        unsafe {
            if let Some(opc) = AluOpCode::from_u16(self.bit_range(21..25) as u16) {
                opc
            } else {
                unreachable_unchecked()
            }
        }
    }

    #[inline(always)]
    fn branch_offset(&self) -> i32 {
        ((self.bit_range(0..24) << 8) as i32) >> 6
    }

    #[inline(always)]
    fn load_flag(&self) -> bool {
        self.bit(20)
    }

    #[inline(always)]
    fn set_cond_flag(&self) -> bool {
        self.bit(20)
    }

    #[inline(always)]
    fn write_back_flag(&self) -> bool {
        self.bit(21)
    }

    #[inline(always)]
    fn accumulate_flag(&self) -> bool {
        self.bit(21)
    }

    #[inline(always)]
    fn u_flag(&self) -> bool {
        self.bit(22)
    }

    #[inline(always)]
    fn halfword_data_transfer_type(&self) -> ArmHalfwordTransferType {
        let bits = (*self & 0b1100000) >> 5;
        ArmHalfwordTransferType::from_u32(bits).unwrap()
    }

    #[inline(always)]
    fn transfer_size(&self) -> usize {
        if self.bit(22) {
            1
        } else {
            4
        }
    }

    #[inline(always)]
    fn psr_and_force_user_flag(&self) -> bool {
        self.bit(22)
    }

    #[inline(always)]
    fn spsr_flag(&self) -> bool {
        self.bit(22)
    }

    #[inline(always)]
    fn add_offset_flag(&self) -> bool {
        self.bit(23)
    }

    #[inline(always)]
    fn pre_index_flag(&self) -> bool {
        self.bit(24)
    }

    #[inline(always)]
    fn link_flag(&self) -> bool {
        self.bit(24)
    }

    /// gets offset used by ldr/str instructions
    #[inline(always)]
    fn ldr_str_offset(&self) -> BarrelShifterValue {
        let ofs = self.bit_range(0..12);
        if self.bit(25) {
            let rm = ofs & 0xf;
            BarrelShifterValue::ShiftedRegister(ShiftedRegister {
                reg: rm as usize,
                shift_by: self.get_shift_reg_by(),
                bs_op: self.get_bs_op(),
                added: Some(self.add_offset_flag()),
            })
        } else {
            let ofs = if self.add_offset_flag() {
                ofs as u32
            } else {
                -(ofs as i32) as u32
            };
            BarrelShifterValue::ImmediateValue(ofs)
        }
    }

    #[inline(always)]
    fn get_bs_op(&self) -> BarrelShiftOpCode {
        BarrelShiftOpCode::from_u8(self.bit_range(5..7) as u8).unwrap()
    }

    #[inline(always)]
    fn get_shift_reg_by(&self) -> ShiftRegisterBy {
        if self.bit(4) {
            let rs = self.bit_range(8..12) as usize;
            ShiftRegisterBy::ByRegister(rs)
        } else {
            let amount = self.bit_range(7..12) as u32;
            ShiftRegisterBy::ByAmount(amount)
        }
    }

    #[inline(always)]
    fn ldr_str_hs_imm_offset(&self) -> BarrelShifterValue {
        let offset8 = (self.bit_range(8..12) << 4) + self.bit_range(0..4);
        let offset8 = if self.add_offset_flag() {
            offset8
        } else {
            (-(offset8 as i32)) as u32
        };
        BarrelShifterValue::ImmediateValue(offset8)
    }

    #[inline(always)]
    fn ldr_str_hs_reg_offset(&self) -> BarrelShifterValue {
        BarrelShifterValue::ShiftedRegister(ShiftedRegister {
            reg: (self & 0xf) as usize,
            shift_by: ShiftRegisterBy::ByAmount(0),
            bs_op: BarrelShiftOpCode::LSL,
            added: Some(self.add_offset_flag()),
        })
    }

    fn operand2(&self) -> BarrelShifterValue {
        if self.bit(25) {
            let immediate = self & 0xff;
            let rotate = 2 * self.bit_range(8..12);
            BarrelShifterValue::RotatedImmediate(immediate, rotate)
        } else {
            let reg = self & 0xf;
            let shifted_reg = ShiftedRegister {
                reg: reg as usize,
                bs_op: self.get_bs_op(),
                shift_by: self.get_shift_reg_by(),
                added: None,
            }; // TODO error handling
            BarrelShifterValue::ShiftedRegister(shifted_reg)
        }
    }

    fn register_list(&self) -> u16 {
        (self & 0xffff) as u16
    }

    fn swi_comment(&self) -> u32 {
        self.bit_range(0..24)
    }
}

#[derive(Debug, PartialEq, Primitive)]
pub enum ArmHalfwordTransferType {
    UnsignedHalfwords = 0b01,
    SignedByte = 0b10,
    SignedHalfwords = 0b11,
}

#[derive(Debug, Copy, Clone, PartialEq, Primitive)]
pub enum ArmCond {
    EQ = 0b0000,
    NE = 0b0001,
    HS = 0b0010,
    LO = 0b0011,
    MI = 0b0100,
    PL = 0b0101,
    VS = 0b0110,
    VC = 0b0111,
    HI = 0b1000,
    LS = 0b1001,
    GE = 0b1010,
    LT = 0b1011,
    GT = 0b1100,
    LE = 0b1101,
    AL = 0b1110,
    Invalid = 0b1111,
}
