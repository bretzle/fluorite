// use fluorite_common::{BitIndex, BitIndexEx};
use std::{
    fs::File,
    io::{self, Write},
    path::Path,
};

fn main() {
    let out_dir = std::env::var_os("OUT_DIR").unwrap().into_string().unwrap();

    generate_cond_lut(format!("{out_dir}/cond_lut.rs")).expect("Failed to generate condition LUT");
    generate_arm_lut(format!("{out_dir}/arm_lut.rs")).expect("Failed to generate arm LUT");
    generate_thumb_lut(format!("{out_dir}/thumb_lut.rs")).expect("Failed to generate thumb LUT");

    println!("cargo:rerun-if-changed=build.rs")
}

fn generate_cond_lut<P: AsRef<Path>>(path: P) -> io::Result<()> {
    let mut file = File::create(path)?;

    let mut lut = [false; 256];
    for flags in 0..=0xF {
        for condition in 0..=0xF {
            let n = flags & 0x8 != 0;
            let z = flags & 0x4 != 0;
            let c = flags & 0x2 != 0;
            let v = flags & 0x1 != 0;
            lut[flags << 4 | condition] = match condition {
                0x0 => z,
                0x1 => !z,
                0x2 => c,
                0x3 => !c,
                0x4 => n,
                0x5 => !n,
                0x6 => v,
                0x7 => !v,
                0x8 => c && !z,
                0x9 => !c || z,
                0xA => n == v,
                0xB => n != v,
                0xC => !z && n == v,
                0xD => z || n != v,
                0xE => true,
                0xF => false, // TODO: Change
                _ => unreachable!(),
            };
        }
    }

    writeln!(file, "pub static CONDITION_LUT: [bool; 256] = [")?;

    for x in lut {
        writeln!(file, "\t{x},")?;
    }

    writeln!(file, "];")?;

    Ok(())
}

macro_rules! bit {
    ($val:expr, $idx:expr) => {
        ($val >> $idx & 0x1 != 0)
    };
}

fn generate_arm_lut<P: AsRef<Path>>(path: P) -> io::Result<()> {
    let mut file = File::create(path)?;

    writeln!(file, "static ARM_LUT: [InstructionHandler<u32>; 4096] = [")?;

    // Bits 0-3 of opcode = Bits 4-7 of instr
    // Bits 4-11 of opcode = Bits Bits 20-27 of instr

    for opcode in 0..4096 {
        let inst = ((opcode & 0xFF0) << 16) | ((opcode & 0xF) << 4);
        let output = if inst & 0xFF000F0 == 0x1200010 {
            "branch_and_exchange".to_string()
        } else if inst & 0xFC000F0 == 0x90 {
            // compose_instr_handler!(mul_mula, skeleton, 21, 20)
            format!("mul_mula::<{}, {}>", bit!(inst, 21), bit!(inst, 20),)
        } else if inst & 0xF8000F0 == 0x800090 {
            // compose_instr_handler!(mul_long, skeleton, 22, 21, 20)
            format!(
                "mul_long::<{}, {}, {}>",
                bit!(inst, 22),
                bit!(inst, 21),
                bit!(inst, 20),
            )
        } else if inst & 0xF800FF0 == 0x1000090 {
            // compose_instr_handler!(single_data_swap, skeleton, 22)
            format!("single_data_swap::<{}>", bit!(inst, 22))
        } else if inst & 0xE000090 == 0x90 {
            // compose_instr_handler!(
            //     halfword_and_signed_data_transfer,
            //     skeleton,
            //     24,
            //     23,
            //     22,
            //     21,
            //     20,
            //     6,
            //     5
            // )
            format!(
                "halfword_and_signed_data_transfer::<{}, {}, {}, {}, {}, {} ,{}>",
                bit!(inst, 24),
                bit!(inst, 23),
                bit!(inst, 22),
                bit!(inst, 21),
                bit!(inst, 20),
                bit!(inst, 6),
                bit!(inst, 5),
            )
        } else if inst & 0xD900000 == 0x1000000 {
            // compose_instr_handler!(psr_transfer, skeleton, 25, 22, 21)
            format!(
                "psr_transfer::<{}, {}, {}>",
                bit!(inst, 25),
                bit!(inst, 22),
                bit!(inst, 21)
            )
        } else if inst & 0xC000000 == 0x0 {
            // compose_instr_handler!(data_proc, skeleton, 25, 20)
            format!("data_proc::<{}, {}>", bit!(inst, 25), bit!(inst, 20),)
        } else if inst & 0xC000000 == 0x4000000 {
            // compose_instr_handler!(single_data_transfer, skeleton, 25, 24, 23, 22, 21, 20)
            format!(
                "single_data_transfer::<{}, {}, {}, {}, {}, {}>",
                bit!(inst, 25),
                bit!(inst, 24),
                bit!(inst, 23),
                bit!(inst, 22),
                bit!(inst, 21),
                bit!(inst, 20)
            )
        } else if inst & 0xE000000 == 0x8000000 {
            // compose_instr_handler!(block_data_transfer, skeleton, 24, 23, 22, 21, 20)
            format!(
                "block_data_transfer::<{}, {}, {}, {}, {}>",
                bit!(inst, 24),
                bit!(inst, 23),
                bit!(inst, 22),
                bit!(inst, 21),
                bit!(inst, 20)
            )
        } else if inst & 0xE000000 == 0xA000000 {
            // compose_instr_handler!(branch_branch_with_link, skeleton, 24)
            format!("branch_branch_with_link::<{}>", bit!(inst, 24))
        } else if inst & 0xF000000 == 0xF000000 {
            "arm_software_interrupt".to_string()
        } else if inst & 0xE000000 == 0xC000000 {
            "coprocessor".to_string()
        } else if inst & 0xF000000 == 0xE000000 {
            "coprocessor".to_string()
        } else {
            assert_eq!(
                inst & 0b1110_0000_0000_0000_0000_0001_0000,
                0b1110_0000_0000_0000_0000_0001_0000
            );
            "undefined_instr_arm".to_string()
        };

        writeln!(file, "\tArm7tdmi::{},", output)?;
    }

    writeln!(file, "];")?;

    Ok(())
}

fn generate_thumb_lut<P: AsRef<Path>>(path: P) -> io::Result<()> {
    let mut file = File::create(path)?;

    //     writeln!(file, "impl<Memory: MemoryInterface> Arm7tdmi<Memory> {{")?;
    //     writeln!(file, "\tconst ARM_HANDLERS: [ArmHandler<Memory>; 4096] = [")?;
    //     for key in 0..4096 {
    //         let inst = ((key & 0xFF0) << 16) | ((key & 0xF) << 4);
    //         let handler = decode_arm_entry(inst);
    //         writeln!(file, "\t\tArmHandler(Arm7tdmi::{}),", handler.1)?;
    //     }
    //     writeln!(file, "\t];")?;
    //     writeln!(file, "}}")?;

    Ok(())
}

// fn decode_arm_entry(i: u32) -> (&'static str, String) {
//     const T: bool = true;
//     const F: bool = false;

//     // First, decode the the top-most non-condition bits
//     match i.bit_range(26..28) {
//         0b00 => {
//             /* DataProcessing and friends */
//             let result = match (i.bit_range(23..26), i.bit_range(4..8)) {
//                 (0b000, 0b1001) => {
//                     if 0b0 == i.ibit(22) {
//                         Some((
//                             "Multiply",
//                             format!(
//                                 "arm_mul_mla::<{UPDATE_FLAGS}, {ACCUMULATE}>",
//                                 UPDATE_FLAGS = i.bit(20),
//                                 ACCUMULATE = i.bit(21),
//                             ),
//                         ))
//                     } else {
//                         None
//                     }
//                 }
//                 (0b001, 0b1001) => Some((
//                     "MultiplyLong",
//                     format!(
//                         "arm_mull_mlal::<{UPDATE_FLAGS}, {ACCUMULATE}, {U_FLAG}>",
//                         UPDATE_FLAGS = i.bit(20),
//                         ACCUMULATE = i.bit(21),
//                         U_FLAG = i.bit(22),
//                     ),
//                 )),
//                 (0b010, 0b1001) => {
//                     if 0b00 == i.bit_range(20..22) {
//                         Some((
//                             "SingleDataSwap",
//                             format!("arm_swp::<{BYTE}>", BYTE = i.bit(22)),
//                         ))
//                     } else {
//                         None
//                     }
//                 }
//                 (0b010, 0b0001) => {
//                     if 0b010 == i.bit_range(20..23) {
//                         Some(("BranchExchange", format!("arm_bx")))
//                     } else {
//                         None
//                     }
//                 }
//                 _ => None,
//             };

//             if let Some(result) = result {
//                 result
//             } else {
//                 match (i.ibit(25), i.ibit(22), i.ibit(7), i.ibit(4)) {
//                     (0, 0, 1, 1) => (
//                         "HalfwordDataTransferRegOffset",
//                         format!(
//                             "arm_ldr_str_hs_reg::<{HS}, {LOAD}, {WRITEBACK}, {PRE_INDEX}, {ADD}>",
//                             HS = (i & 0b1100000) >> 5,
//                             LOAD = i.bit(20),
//                             WRITEBACK = i.bit(21),
//                             ADD = i.bit(23),
//                             PRE_INDEX = i.bit(24),
//                         ),
//                     ),
//                     (0, 1, 1, 1) => (
//                         "HalfwordDataTransferImmediateOffset",
//                         format!(
//                             "arm_ldr_str_hs_imm::<{HS}, {LOAD}, {WRITEBACK}, {PRE_INDEX}, {ADD}>",
//                             HS = (i & 0b1100000) >> 5,
//                             LOAD = i.bit(20),
//                             WRITEBACK = i.bit(21),
//                             ADD = i.bit(23),
//                             PRE_INDEX = i.bit(24)
//                         ),
//                     ),
//                     _ => {
//                         let set_cond_flags = i.bit(20);
//                         // PSR Transfers are encoded as a subset of Data Processing,
//                         // with S bit OFF and the encode opcode is one of TEQ,CMN,TST,CMN
//                         let is_op_not_touching_rd = i.bit_range(21..25) & 0b1100 == 0b1000;
//                         if !set_cond_flags && is_op_not_touching_rd {
//                             if i.bit(21) {
//                                 (
//                                     "MoveToStatus",
//                                     format!(
//                                         "arm_transfer_to_status::<{IMM}, {SPSR_FLAG}>",
//                                         IMM = i.bit(25),
//                                         SPSR_FLAG = i.bit(22)
//                                     ),
//                                 )
//                             } else {
//                                 (
//                                     "MoveFromStatus",
//                                     format!("arm_mrs::<{SPSR_FLAG}>", SPSR_FLAG = i.bit(22)),
//                                 )
//                             }
//                         } else {
//                             ("DataProcessing", format!("arm_data_processing::<{OP}, {IMM}, {SET_FLAGS}, {SHIFT_BY_REG}>",
//                                 OP=i.bit_range(21..25),
//                                 IMM=i.bit(25),
//                                 SET_FLAGS=i.bit(20),
//                                 SHIFT_BY_REG=i.bit(4)))
//                         }
//                     }
//                 }
//             }
//         }
//         0b01 => {
//             match (i.bit(25), i.bit(4)) {
//                 (_, F) | (F, T) => ("SingleDataTransfer", format!(
//                     "arm_ldr_str::<{LOAD}, {WRITEBACK}, {PRE_INDEX}, {BYTE}, {SHIFT}, {ADD}, {BS_OP}, {SHIFT_BY_REG}>",
//                     LOAD = i.bit(20),
//                     WRITEBACK = i.bit(21),
//                     BYTE = i.bit(22),
//                     ADD = i.bit(23),
//                     PRE_INDEX = i.bit(24),
//                     SHIFT = i.bit(25),
//                     BS_OP = i.bit_range(5..7) as u8,
//                     SHIFT_BY_REG = i.bit(4),
//                 )),
//                 (T, T) => ("Undefined", String::from("arm_undefined")), /* Possible ARM11 but we don't implement these */
//             }
//         }
//         0b10 => match i.bit(25) {
//             F => (
//                 "BlockDataTransfer",
//                 format!(
//                     "arm_ldm_stm::<{LOAD}, {WRITEBACK}, {FLAG_S}, {ADD}, {PRE_INDEX}>",
//                     LOAD = i.bit(20),
//                     WRITEBACK = i.bit(21),
//                     FLAG_S = i.bit(22),
//                     ADD = i.bit(23),
//                     PRE_INDEX = i.bit(24),
//                 ),
//             ),
//             T => (
//                 "BranchLink",
//                 format!("arm_b_bl::<{LINK}>", LINK = i.bit(24)),
//             ),
//         },
//         0b11 => {
//             match (i.ibit(25), i.ibit(24), i.ibit(4)) {
//                 (0b0, _, _) => ("Undefined", String::from("arm_undefined")), /* CoprocessorDataTransfer not implemented */
//                 (0b1, 0b0, 0b0) => ("Undefined", String::from("arm_undefined")), /* CoprocessorDataOperation not implemented */
//                 (0b1, 0b0, 0b1) => ("Undefined", String::from("arm_undefined")), /* CoprocessorRegisterTransfer not implemented */
//                 (0b1, 0b1, _) => ("SoftwareInterrupt", String::from("arm_swi")),
//                 _ => ("Undefined", String::from("arm_undefined")),
//             }
//         }
//         _ => unreachable!(),
//     }
// }
