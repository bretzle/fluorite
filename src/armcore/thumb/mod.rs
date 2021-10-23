mod exec;

use crate::armcore::Arm7tdmi;
use exec::*;

include!(concat!(env!("OUT_DIR"), "/thumb_lut.rs"));

pub struct ThumbInstruction {
    handler: fn(&mut Arm7tdmi, u16),
    typ: ThumbInstructionType,
}

pub enum ThumbInstructionType {
    AddSub,
    MoveShiftedReg,
    DataProcessImm,
    AluOps,
    HiRegOpOrBranchExchange,
    LdrPc,
    LdrStrRegOffset,
    LdrStrSHB,
    LdrStrImmOffset,
    LdrStrHalfWord,
    LdrStrSp,
    LoadAddress,
    AddSp,
    PushPop,
    LdmStm,
    Swi,
    BranchConditional,
    Branch,
    BranchLongWithLink,
    Unknown,
}

impl ThumbInstruction {
    pub fn execute(&self, arm: &mut Arm7tdmi, code: u16) {
        (self.handler)(arm, code)
    }
}
