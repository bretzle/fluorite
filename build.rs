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
                0xF => false,
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

    writeln!(
        file,
        "static THUMB_LUT: [InstructionHandler<u16>; 256] = ["
    )?;

    // Bits 0-7 of opcode = Bits 16-31 of instr

    for opcode in 0..256 {
        let inst = opcode << 8;

        let output = if opcode & 0b1111_1000 == 0b0001_1000 {
            format!("add_sub::<{}, {}>", bit!(inst, 10), bit!(inst, 9))
        } else if opcode & 0b1110_0000 == 0b0000_0000 {
            format!("move_shifted_reg::<{}, {}>", bit!(inst, 12), bit!(inst, 11))
        } else if opcode & 0b1110_0000 == 0b0010_0000 {
            format!(
                "immediate::<{}, {}, {}, {}, {}>",
                bit!(inst, 12),
                bit!(inst, 11),
                bit!(inst, 10),
                bit!(inst, 9),
                bit!(inst, 8)
            )
        } else if opcode & 0b1111_1100 == 0b0100_0000 {
            "alu".to_string()
        } else if opcode & 0b1111_1100 == 0b0100_0100 {
            format!("hi_reg_bx::<{}, {}>", bit!(inst, 9), bit!(inst, 8))
        } else if opcode & 0b1111_1000 == 0b0100_1000 {
            format!(
                "load_pc_rel::<{}, {}, {}>",
                bit!(inst, 10),
                bit!(inst, 9),
                bit!(inst, 8)
            )
        } else if opcode & 0b1111_0010 == 0b0101_0000 {
            format!(
                "load_store_reg_offset::<{}, {}>",
                bit!(inst, 11),
                bit!(inst, 10)
            )
        } else if opcode & 0b1111_0010 == 0b0101_0010 {
            format!(
                "load_store_sign_ext::<{}, {}>",
                bit!(inst, 11),
                bit!(inst, 10)
            )
        } else if opcode & 0b1110_0000 == 0b0110_0000 {
            format!(
                "load_store_imm_offset::<{}, {}>",
                bit!(inst, 12),
                bit!(inst, 11)
            )
        } else if opcode & 0b1111_0000 == 0b1000_0000 {
            format!("load_store_halfword::<{}>", bit!(inst, 11))
        } else if opcode & 0b1111_0000 == 0b1001_0000 {
            format!(
                "load_store_sp_rel::<{}, {}, {}, {}>",
                bit!(inst, 11),
                bit!(inst, 10),
                bit!(inst, 9),
                bit!(inst, 8)
            )
        } else if opcode & 0b1111_0000 == 0b1010_0000 {
            format!(
                "get_rel_addr::<{}, {}, {}, {}>",
                bit!(inst, 11),
                bit!(inst, 10),
                bit!(inst, 9),
                bit!(inst, 8)
            )
        } else if opcode & 0b1111_1111 == 0b1011_0000 {
            "add_offset_sp".to_string()
        } else if opcode & 0b1111_0110 == 0b1011_0100 {
            format!("push_pop_regs::<{}, {}>", bit!(inst, 11), bit!(inst, 8))
        } else if opcode & 0b1111_0000 == 0b1100_0000 {
            format!(
                "multiple_load_store::<{}, {}, {}, {}>",
                bit!(inst, 11),
                bit!(inst, 10),
                bit!(inst, 9),
                bit!(inst, 8)
            )
        } else if opcode & 0b1111_1111 == 0b1101_1111 {
            "thumb_software_interrupt".to_string()
        } else if opcode & 0b1111_0000 == 0b1101_0000 {
            format!(
                "cond_branch::<{}, {}, {}, {}>",
                bit!(inst, 11),
                bit!(inst, 10),
                bit!(inst, 9),
                bit!(inst, 8)
            )
        } else if opcode & 0b1111_1000 == 0b1110_0000 {
            "uncond_branch".to_string()
        } else if opcode & 0b1111_0000 == 0b1111_0000 {
            format!("branch_with_link::<{}>", bit!(inst, 11))
        } else {
            "undefined_instr_thumb".to_string()
        };

        writeln!(file, "\tArm7tdmi::{},", output)?;
    }

    writeln!(file, "];")?;

    Ok(())
}
