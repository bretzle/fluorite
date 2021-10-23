use enum_primitive_derive::*;

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
