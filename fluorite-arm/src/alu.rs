use enum_primitive_derive::*;
use fluorite_common::BitIndex;

use crate::{cpu::Arm7tdmi, memory::MemoryInterface, REG_PC};

#[derive(Debug, PartialEq, Copy, Clone, Primitive)]
pub enum BarrelShiftOpCode {
    LSL = 0,
    LSR = 1,
    ASR = 2,
    ROR = 3,
}

#[derive(Debug, Eq, PartialEq, Primitive)]
pub enum AluOpCode {
    AND = 0b0000,
    EOR = 0b0001,
    SUB = 0b0010,
    RSB = 0b0011,
    ADD = 0b0100,
    ADC = 0b0101,
    SBC = 0b0110,
    RSC = 0b0111,
    TST = 0b1000,
    TEQ = 0b1001,
    CMP = 0b1010,
    CMN = 0b1011,
    ORR = 0b1100,
    MOV = 0b1101,
    BIC = 0b1110,
    MVN = 0b1111,
}

impl AluOpCode {
    pub fn is_settings_flags(&self) -> bool {
        use AluOpCode::*;
        match self {
            TST | TEQ | CMP | CMN => true,
            _ => false,
        }
    }

    pub fn is_logical(&self) -> bool {
        use AluOpCode::*;
        match self {
            MOV | MVN | ORR | EOR | AND | BIC | TST | TEQ => true,
            _ => false,
        }
    }

    pub fn is_arithmetic(&self) -> bool {
        use AluOpCode::*;
        match self {
            ADD | ADC | SUB | SBC | RSB | RSC | CMP | CMN => true,
            _ => false,
        }
    }
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum ShiftRegisterBy {
    ByAmount(u32),
    ByRegister(usize),
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum BarrelShifterValue {
    ImmediateValue(u32),
    RotatedImmediate(u32, u32),
    ShiftedRegister(ShiftedRegister),
}

impl BarrelShifterValue {
    pub fn decode_rotated_immediate(&self) -> Option<u32> {
        if let BarrelShifterValue::RotatedImmediate(imm, rotate) = self {
            return Some(imm.rotate_right(*rotate) as u32);
        }
        None
    }
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub struct ShiftedRegister {
    pub reg: usize,
    pub shift_by: ShiftRegisterBy,
    pub bs_op: BarrelShiftOpCode,
    pub added: Option<bool>,
}

impl<Memory: MemoryInterface> Arm7tdmi<Memory> {
    fn rrx(&mut self, val: u32, carry: &mut bool) -> u32 {
        let old = *carry as i32;
        *carry = val & 1 != 0;
        (((val as u32) >> 1) as i32 | (old << 31)) as u32
    }

    pub(crate) fn lsl(&mut self, val: u32, amount: u32, carry: &mut bool) -> u32 {
        match amount {
            0 => val,
            x if x < 32 => {
                *carry = val.wrapping_shr(32 - x) & 1 == 1;
                val << x
            }
            32 => {
                *carry = val & 1 == 1;
                0
            }
            _ => {
                *carry = false;
                0
            }
        }
    }

    pub(crate) fn lsr(&mut self, val: u32, amount: u32, carry: &mut bool, immediate: bool) -> u32 {
        if amount != 0 {
            match amount {
                x if x < 32 => {
                    *carry = (val >> (amount - 1) & 1) == 1;
                    val >> amount
                }
                32 => {
                    *carry = val.bit(31);
                    0
                }
                _ => {
                    *carry = false;
                    0
                }
            }
        } else if immediate {
            *carry = val.bit(31);
            0
        } else {
            val
        }
    }

    pub(crate) fn asr(&mut self, val: u32, amount: u32, carry: &mut bool, immediate: bool) -> u32 {
        let amount = if immediate && amount == 0 { 32 } else { amount };
        match amount {
            0 => val,
            x if x < 32 => {
                *carry = val.wrapping_shr(amount - 1) & 1 == 1;
                (val as i32).wrapping_shr(amount) as u32
            }
            _ => {
                let bit31 = val.bit(31);
                *carry = bit31;
                if bit31 {
                    0xffffffff
                } else {
                    0
                }
            }
        }
    }

    pub(crate) fn ror(
        &mut self,
        val: u32,
        amount: u32,
        carry: &mut bool,
        immediate: bool,
        rrx: bool,
    ) -> u32 {
        match amount {
            0 => {
                if immediate & rrx {
                    self.rrx(val, carry)
                } else {
                    val
                }
            }
            _ => {
                let amount = amount % 32;
                let val = if amount != 0 {
                    val.rotate_right(amount)
                } else {
                    val
                };
                *carry = val.bit(31);
                val
            }
        }
    }

    pub(crate) fn register_shift(&mut self, shift: &ShiftedRegister, carry: &mut bool) -> u32 {
        match shift.shift_by {
            ShiftRegisterBy::ByAmount(amount) => {
                let ret =
                    self.barrel_shift_op(shift.bs_op, self.get_reg(shift.reg), amount, carry, true);
                ret
            }
            ShiftRegisterBy::ByRegister(rs) => {
                self.shift_by_register(shift.bs_op, shift.reg, rs, carry)
            }
        }
    }

    fn barrel_shift_op(
        &mut self,
        shift: BarrelShiftOpCode,
        val: u32,
        amount: u32,
        carry: &mut bool,
        immediate: bool,
    ) -> u32 {
        match shift {
            BarrelShiftOpCode::LSL => self.lsl(val, amount, carry),
            BarrelShiftOpCode::LSR => self.lsr(val, amount, carry, immediate),
            BarrelShiftOpCode::ASR => self.asr(val, amount, carry, immediate),
            BarrelShiftOpCode::ROR => self.ror(val, amount, carry, immediate, true),
        }
    }

    pub fn shift_by_register(
        &mut self,
        bs_op: BarrelShiftOpCode,
        reg: usize,
        rs: usize,
        carry: &mut bool,
    ) -> u32 {
        let mut val = self.get_reg(reg);
        if reg == REG_PC {
            val += 4; // PC prefetching
        }
        let amount = self.get_reg(rs) & 0xff;
        self.barrel_shift_op(bs_op, val, amount, carry, false)
    }

    fn register_shift_const<const BS_OP: u8, const SHIFT_BY_REG: bool>(
        &mut self,
        offset: u32,
        reg: usize,
        carry: &mut bool,
    ) -> u32 {
        let op = match BS_OP {
            0 => BarrelShiftOpCode::LSL,
            1 => BarrelShiftOpCode::LSR,
            2 => BarrelShiftOpCode::ASR,
            3 => BarrelShiftOpCode::ROR,
            _ => unsafe { std::hint::unreachable_unchecked() },
        };
        if SHIFT_BY_REG {
            let rs = offset.bit_range(8..12) as usize;
            self.shift_by_register(op, reg, rs, carry)
        } else {
            let amount = offset.bit_range(7..12) as u32;
            self.barrel_shift_op(op, self.get_reg(reg), amount, carry, true)
        }
    }

    pub(crate) fn alu_update_flags(&mut self, result: u32, _is_arithmetic: bool, c: bool, v: bool) {
        self.cspr.set_n((result as i32) < 0);
        self.cspr.set_z(result == 0);
        self.cspr.set_c(c);
        self.cspr.set_v(v);
    }
}
