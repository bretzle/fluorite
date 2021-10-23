use crate::armcore::Arm7tdmi;

pub fn thumb_move_shifted_reg<const BS_OP: u8, const IMM: u8>(arm: &mut Arm7tdmi, inst: u16) {
    todo!()
}

pub fn thumb_add_sub<const SUB: bool, const IMM: bool, const RN: usize>(
    arm: &mut Arm7tdmi,
    inst: u16,
) {
    todo!()
}

pub fn thumb_data_process_imm<const OP: u8, const RD: usize>(arm: &mut Arm7tdmi, inst: u16) {
    todo!()
}

pub fn thumb_alu_ops<const OP: u16>(arm: &mut Arm7tdmi, inst: u16) {
    todo!()
}

pub fn thumb_hi_reg_op_or_bx<const OP: u8, const FLAG_H1: bool, const FLAG_H2: bool>(
    arm: &mut Arm7tdmi,
    inst: u16,
) {
    todo!()
}

pub fn thumb_ldr_pc<const RD: usize>(arm: &mut Arm7tdmi, inst: u16) {
    todo!()
}

pub fn thumb_ldr_str_reg_offset<const LOAD: bool, const RO: usize, const BYTE: bool>(
    arm: &mut Arm7tdmi,
    inst: u16,
) {
    todo!()
}

pub fn thumb_ldr_str_shb<const RO: usize, const SIGN_EXTEND: bool, const HALFWORD: bool>(
    arm: &mut Arm7tdmi,
    inst: u16,
) {
    todo!()
}

pub fn thumb_ldr_str_imm_offset<const LOAD: bool, const BYTE: bool, const OFFSET: u8>(
    arm: &mut Arm7tdmi,
    inst: u16,
) {
    todo!()
}

pub fn thumb_ldr_str_halfword<const LOAD: bool, const OFFSET: i32>(arm: &mut Arm7tdmi, inst: u16) {
    todo!()
}

pub fn thumb_ldr_str_sp<const LOAD: bool, const RD: usize>(arm: &mut Arm7tdmi, inst: u16) {
    todo!()
}

pub fn thumb_load_address<const SP: bool, const RD: usize>(arm: &mut Arm7tdmi, inst: u16) {
    todo!()
}

pub fn thumb_add_sp<const FLAG_S: bool>(arm: &mut Arm7tdmi, inst: u16) {
    todo!()
}

pub fn thumb_push_pop<const POP: bool, const FLAG_R: bool>(arm: &mut Arm7tdmi, inst: u16) {
    todo!()
}

pub fn thumb_ldm_stm<const LOAD: bool, const RB: usize>(arm: &mut Arm7tdmi, inst: u16) {
    todo!()
}

pub fn thumb_branch_with_cond<const COND: u8>(arm: &mut Arm7tdmi, inst: u16) {
    todo!()
}

pub fn thumb_swi(arm: &mut Arm7tdmi, _inst: u16) {
    todo!()
}

pub fn thumb_branch(arm: &mut Arm7tdmi, inst: u16) {
    todo!()
}

pub fn thumb_branch_long_with_link<const FLAG_LOW_OFFSET: bool>(arm: &mut Arm7tdmi, inst: u16) {
    todo!()
}

pub fn thumb_unknown(arm: &mut Arm7tdmi, inst: u16) {
    panic!(
        "executing undefind thumb instruction {:04x} at @{:08x}",
        inst, arm.pc
    )
}
