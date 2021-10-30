use fluorite_arm::thumb::ThumbInstruction;
use fluorite_arm::InstructionDecoder;

#[test]
fn mov_low_reg() {
    let insn = ThumbInstruction::decode(0x2027, 0);
    assert_eq!(format!("{}", insn), "mov\tr0, #0x27");
}

#[test]
fn ldr_pc() {
    let insn = ThumbInstruction::decode(0x4801, 0x6);
    assert_eq!(format!("{}", insn), "ldr\tr0, [pc, #0x4] ; = #0xc");
}

#[test]
fn ldr_str_reg_offset() {
    let str_insn = ThumbInstruction::decode(0x5060, 0x6);
    let ldr_insn = ThumbInstruction::decode(0x5c62, 0x6);

    assert_eq!(format!("{}", str_insn), "str\tr0, [r4, r1]");
    assert_eq!(format!("{}", ldr_insn), "ldrb\tr2, [r4, r1]");
}

#[allow(overflowing_literals)]
#[test]
fn format8() {
    let decoded = ThumbInstruction::decode(0x521c, 0);
    assert_eq!(format!("{}", decoded), "strh\tr4, [r3, r0]");

    let decoded = ThumbInstruction::decode(0x567a, 0);
    assert_eq!(format!("{}", decoded), "ldsb\tr2, [r7, r1]");

    let decoded = ThumbInstruction::decode(0x5ea3, 0);
    assert_eq!(format!("{}", decoded), "ldsh\tr3, [r4, r2]");
}
