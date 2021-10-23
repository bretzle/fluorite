mod alu;
pub mod arm;
pub mod cpu;
mod registers;
pub mod thumb;

use arm::ArmInstruction;
use std::fmt;
use thumb::ThumbInstruction;

pub type Addr = u32;

pub const REG_PC: usize = 15;
pub const REG_LR: usize = 14;
pub const REG_SP: usize = 13;

#[derive(Debug, PartialEq, Clone)]
pub enum DecodedInstruction {
    Arm(ArmInstruction),
    Thumb(ThumbInstruction),
}

impl DecodedInstruction {
    pub fn get_pc(&self) -> Addr {
        match self {
            DecodedInstruction::Arm(a) => a.pc,
            DecodedInstruction::Thumb(t) => t.pc,
        }
    }
}

impl fmt::Display for DecodedInstruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DecodedInstruction::Arm(a) => write!(f, "{}", a),
            DecodedInstruction::Thumb(t) => write!(f, "{}", t),
        }
    }
}

pub trait InstructionDecoder {
    type IntType;

    fn decode(n: Self::IntType, addr: Addr) -> Self;
    fn decode_bytes(bytes: &[u8], addr: Addr) -> Self;
    fn get_raw(&self) -> Self::IntType;
}

pub fn reg_string<T: Into<usize>>(reg: T) -> &'static str {
    let reg_names = &[
        "r0", "r1", "r2", "r3", "r4", "r5", "r6", "r7", "r8", "r9", "r10", "fp", "ip", "sp", "lr",
        "pc",
    ];
    reg_names[reg.into()]
}
