use fluorite_common::BitIndex;
use std::{
    env,
    fs::File,
    io::{self, Write},
    path::Path,
};

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let thumb_path = Path::new(&out_dir).join("thumb_lut.rs");
    let mut thumb_file = File::create(thumb_path).unwrap();
    generate_thumb_lut(&mut thumb_file).unwrap();

    println!("cargo:rerun-if-changed=build.rs")
}

fn generate_thumb_lut(file: &mut File) -> io::Result<()> {
    writeln!(file, "pub static THUMB_LUT: [ThumbInstruction; 1024] = [")?;

    for i in 0..1024 {
        let (handler, typ) = thumb_decode(i << 6);
        writeln!(
            file,
            "\tThumbInstruction {{ handler:{}, typ: ThumbInstructionType::{} }},",
            handler, typ
        )?;
    }

    writeln!(file, "];")?;

    Ok(())
}

fn thumb_decode(i: u32) -> (String, &'static str) {
    let offset5 = i.bit_range(6..11) as u8;
    let load = i.bit(11);

    if i & 0xF800 == 0x1800 {
        (
            format!(
                "thumb_add_sub::<{SUB}, {IMM}, {RN}>",
                SUB = i.bit(9),
                IMM = i.bit(10),
                RN = i.bit_range(6..9) as usize
            ),
            "AddSub",
        )
    } else if i & 0xE000 == 0x0000 {
        (
            format!(
                "thumb_move_shifted_reg::<{BS_OP}, {IMM}>",
                BS_OP = i.bit_range(11..13) as u8,
                IMM = i.bit_range(6..11) as u8
            ),
            "MoveShiftedReg",
        )
    } else if i & 0xE000 == 0x2000 {
        (
            format!(
                "thumb_data_process_imm::<{OP}, {RD}>",
                OP = i.bit_range(11..13) as u8,
                RD = i.bit_range(8..11)
            ),
            "DataProcessImm",
        )
    } else if i & 0xFC00 == 0x4000 {
        (
            format!("thumb_alu_ops::<{OP}>", OP = i.bit_range(6..10) as u16),
            "AluOps",
        )
    } else if i & 0xFC00 == 0x4400 {
        (
            format!(
                "thumb_hi_reg_op_or_bx::<{OP}, {FLAG_H1}, {FLAG_H2}>",
                OP = i.bit_range(8..10) as u8,
                FLAG_H1 = i.bit(7),
                FLAG_H2 = i.bit(6),
            ),
            "HiRegOpOrBranchExchange",
        )
    } else if i & 0xF800 == 0x4800 {
        (
            format!("thumb_ldr_pc::<{RD}>", RD = i.bit_range(8..11) as usize),
            "LdrPc",
        )
    } else if i & 0xF200 == 0x5000 {
        (
            format!(
                "thumb_ldr_str_reg_offset::<{LOAD}, {RO}, {BYTE}>",
                LOAD = load,
                RO = i.bit_range(6..9) as usize,
                BYTE = i.bit(10),
            ),
            "LdrStrRegOffset",
        )
    } else if i & 0xF200 == 0x5200 {
        (
            format!(
                "thumb_ldr_str_shb::<{RO}, {SIGN_EXTEND}, {HALFWORD}>",
                RO = i.bit_range(6..9) as usize,
                SIGN_EXTEND = i.bit(10),
                HALFWORD = i.bit(11),
            ),
            "LdrStrSHB",
        )
    } else if i & 0xE000 == 0x6000 {
        let is_transferring_bytes = i.bit(12);
        let offset = if is_transferring_bytes {
            offset5
        } else {
            (offset5 << 3) >> 1
        };

        (
            format!(
                "thumb_ldr_str_imm_offset::<{LOAD}, {BYTE}, {OFFSET}>",
                LOAD = load,
                BYTE = is_transferring_bytes,
                OFFSET = offset
            ),
            "LdrStrImmOffset",
        )
    } else if i & 0xF000 == 0x8000 {
        (
            format!(
                "thumb_ldr_str_halfword::<{LOAD}, {OFFSET}>",
                LOAD = load,
                OFFSET = (offset5 << 1) as i32
            ),
            "LdrStrHalfWord",
        )
    } else if i & 0xF000 == 0x9000 {
        (
            format!(
                "thumb_ldr_str_sp::<{LOAD}, {RD}>",
                LOAD = load,
                RD = i.bit_range(8..11)
            ),
            "LdrStrSp",
        )
    } else if i & 0xF000 == 0xA000 {
        (
            format!(
                "thumb_load_address::<{SP}, {RD}>",
                SP = i.bit(11),
                RD = i.bit_range(8..11)
            ),
            "LoadAddress",
        )
    } else if i & 0xFF00 == 0xB000 {
        (
            format!("thumb_add_sp::<{FLAG_S}>", FLAG_S = i.bit(7)),
            "AddSp",
        )
    } else if i & 0xF600 == 0xB400 {
        (
            format!(
                "thumb_push_pop::<{POP}, {FLAG_R}>",
                POP = load,
                FLAG_R = i.bit(8)
            ),
            "PushPop",
        )
    } else if i & 0xF000 == 0xC000 {
        (
            format!(
                "thumb_ldm_stm::<{LOAD}, {RB}>",
                LOAD = load,
                RB = i.bit_range(8..11) as usize
            ),
            "LdmStm",
        )
    } else if i & 0xFF00 == 0xDF00 {
        ("thumb_swi".to_string(), "Swi")
    } else if i & 0xF000 == 0xD000 {
        (
            format!(
                "thumb_branch_with_cond::<{COND}>",
                COND = i.bit_range(8..12) as u8
            ),
            "BranchConditional",
        )
    } else if i & 0xF800 == 0xE000 {
        ("thumb_branch".to_string(), "Branch")
    } else if i & 0xF000 == 0xF000 {
        (
            format!(
                "thumb_branch_long_with_link::<{FLAG_LOW_OFFSET}>",
                FLAG_LOW_OFFSET = i.bit(11),
            ),
            "BranchLongWithLink",
        )
    } else {
        ("thumb_unknown".to_string(), "Unknown")
    }
}
