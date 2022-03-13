use fluorite_common::{BitIndex, BitIndexEx};
use std::{
    fs::File,
    io::{self, Write},
    path::Path,
};

fn main() {
    let out_dir = std::env::var_os("OUT_DIR").unwrap();

    let arm_path = Path::new(&out_dir).join("arm_table.rs");
    let mut arm_file = File::create(&arm_path).unwrap();
    generate_arm_table(&mut arm_file).expect("Failed to generate arm LUT");

    println!("cargo:rerun-if-changed=build.rs")
}

fn generate_arm_table(file: &mut File) -> io::Result<()> {
    writeln!(file, "impl<Memory: MemoryInterface> Arm7tdmi<Memory> {{")?;
    writeln!(file, "\tconst ARM_HANDLERS: [ArmHandler<Memory>; 4096] = [")?;
    for key in 0..4096 {
        let inst = ((key & 0xFF0) << 16) | ((key & 0xF) << 4);
        let handler = decode_arm_entry(inst);
        writeln!(file, "\t\tArmHandler(Arm7tdmi::{}),", handler.1)?;
    }
    writeln!(file, "\t];")?;
    writeln!(file, "}}")?;

    Ok(())
}

fn decode_arm_entry(i: u32) -> (&'static str, String) {
    const T: bool = true;
    const F: bool = false;

    // First, decode the the top-most non-condition bits
    match i.bit_range(26..28) {
        0b00 => {
            /* DataProcessing and friends */

            let result = match (i.bit_range(23..26), i.bit_range(4..8)) {
                (0b000, 0b1001) => {
                    if 0b0 == i.ibit(22) {
                        Some((
                            "Multiply",
                            format!(
                                "arm_mul_mla::<{UPDATE_FLAGS}, {ACCUMULATE}>",
                                UPDATE_FLAGS = i.bit(20),
                                ACCUMULATE = i.bit(21),
                            ),
                        ))
                    } else {
                        None
                    }
                }
                (0b001, 0b1001) => Some((
                    "MultiplyLong",
                    format!(
                        "arm_mull_mlal::<{UPDATE_FLAGS}, {ACCUMULATE}, {U_FLAG}>",
                        UPDATE_FLAGS = i.bit(20),
                        ACCUMULATE = i.bit(21),
                        U_FLAG = i.bit(22),
                    ),
                )),
                (0b010, 0b1001) => {
                    if 0b00 == i.bit_range(20..22) {
                        Some((
                            "SingleDataSwap",
                            format!("arm_swp::<{BYTE}>", BYTE = i.bit(22)),
                        ))
                    } else {
                        None
                    }
                }
                (0b010, 0b0001) => {
                    if 0b010 == i.bit_range(20..23) {
                        Some(("BranchExchange", format!("arm_bx")))
                    } else {
                        None
                    }
                }
                _ => None,
            };

            if let Some(result) = result {
                result
            } else {
                match (i.ibit(25), i.ibit(22), i.ibit(7), i.ibit(4)) {
                    (0, 0, 1, 1) => (
                        "HalfwordDataTransferRegOffset",
                        format!(
                            "arm_ldr_str_hs_reg::<{HS}, {LOAD}, {WRITEBACK}, {PRE_INDEX}, {ADD}>",
                            HS = (i & 0b1100000) >> 5,
                            LOAD = i.bit(20),
                            WRITEBACK = i.bit(21),
                            ADD = i.bit(23),
                            PRE_INDEX = i.bit(24),
                        ),
                    ),
                    (0, 1, 1, 1) => (
                        "HalfwordDataTransferImmediateOffset",
                        format!(
                            "arm_ldr_str_hs_imm::<{HS}, {LOAD}, {WRITEBACK}, {PRE_INDEX}, {ADD}>",
                            HS = (i & 0b1100000) >> 5,
                            LOAD = i.bit(20),
                            WRITEBACK = i.bit(21),
                            ADD = i.bit(23),
                            PRE_INDEX = i.bit(24)
                        ),
                    ),
                    _ => {
                        let set_cond_flags = i.bit(20);
                        // PSR Transfers are encoded as a subset of Data Processing,
                        // with S bit OFF and the encode opcode is one of TEQ,CMN,TST,CMN
                        let is_op_not_touching_rd = i.bit_range(21..25) & 0b1100 == 0b1000;
                        if !set_cond_flags && is_op_not_touching_rd {
                            if i.bit(21) {
                                (
                                    "MoveToStatus",
                                    format!(
                                        "arm_transfer_to_status::<{IMM}, {SPSR_FLAG}>",
                                        IMM = i.bit(25),
                                        SPSR_FLAG = i.bit(22)
                                    ),
                                )
                            } else {
                                (
                                    "MoveFromStatus",
                                    format!("arm_mrs::<{SPSR_FLAG}>", SPSR_FLAG = i.bit(22)),
                                )
                            }
                        } else {
                            ("DataProcessing", format!("arm_data_processing::<{OP}, {IMM}, {SET_FLAGS}, {SHIFT_BY_REG}>",
                                OP=i.bit_range(21..25),
                                IMM=i.bit(25),
                                SET_FLAGS=i.bit(20),
                                SHIFT_BY_REG=i.bit(4)))
                        }
                    }
                }
            }
        }
        0b01 => {
            match (i.bit(25), i.bit(4)) {
                (_, F) | (F, T) => ("SingleDataTransfer", format!(
                    "arm_ldr_str::<{LOAD}, {WRITEBACK}, {PRE_INDEX}, {BYTE}, {SHIFT}, {ADD}, {BS_OP}, {SHIFT_BY_REG}>",
                    LOAD = i.bit(20),
                    WRITEBACK = i.bit(21),
                    BYTE = i.bit(22),
                    ADD = i.bit(23),
                    PRE_INDEX = i.bit(24),
                    SHIFT = i.bit(25),
                    BS_OP = i.bit_range(5..7) as u8,
                    SHIFT_BY_REG = i.bit(4),
                )),
                (T, T) => ("Undefined", String::from("arm_undefined")), /* Possible ARM11 but we don't implement these */
            }
        }
        0b10 => match i.bit(25) {
            F => (
                "BlockDataTransfer",
                format!(
                    "arm_ldm_stm::<{LOAD}, {WRITEBACK}, {FLAG_S}, {ADD}, {PRE_INDEX}>",
                    LOAD = i.bit(20),
                    WRITEBACK = i.bit(21),
                    FLAG_S = i.bit(22),
                    ADD = i.bit(23),
                    PRE_INDEX = i.bit(24),
                ),
            ),
            T => (
                "BranchLink",
                format!("arm_b_bl::<{LINK}>", LINK = i.bit(24)),
            ),
        },
        0b11 => {
            match (i.ibit(25), i.ibit(24), i.ibit(4)) {
                (0b0, _, _) => ("Undefined", String::from("arm_undefined")), /* CoprocessorDataTransfer not implemented */
                (0b1, 0b0, 0b0) => ("Undefined", String::from("arm_undefined")), /* CoprocessorDataOperation not implemented */
                (0b1, 0b0, 0b1) => ("Undefined", String::from("arm_undefined")), /* CoprocessorRegisterTransfer not implemented */
                (0b1, 0b1, _) => ("SoftwareInterrupt", String::from("arm_swi")),
                _ => ("Undefined", String::from("arm_undefined")),
            }
        }
        _ => unreachable!(),
    }
}
