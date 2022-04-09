use crate::arm::registers::Mode;
use crate::arm::registers::Reg;
use crate::arm::Arm7tdmi;
use crate::arm::InstructionHandler;
use crate::arm::CONDITION_LUT;
use crate::io::{MemoryAccess, Sysbus};

use super::DataOp;

include!(concat!(env!("OUT_DIR"), "/arm_lut.rs"));

impl Arm7tdmi {
    pub(super) fn fill_arm_instr_buffer(&mut self, bus: &mut Sysbus) {
        self.regs.pc &= !0x3;
        self.pipeline[0] = self.read::<u32>(bus, MemoryAccess::S, self.regs.pc & !0x3);
        self.regs.pc = self.regs.pc.wrapping_add(4);

        self.pipeline[1] = self.read::<u32>(bus, MemoryAccess::S, self.regs.pc & !0x3);
    }

    #[inline]
    #[cfg(feature = "decode")]
    pub fn decode_arm_instr(addr: u32, instr: u32) -> String {
        use yaxpeax_arch::{Arch, Decoder, U8Reader};
        use yaxpeax_arm::armv7::ARMv7;

        let data = [
            (instr & 0xFF) as u8,
            (instr >> 8 & 0xFF) as u8,
            (instr >> 16 & 0xFF) as u8,
            (instr >> 24 & 0xFF) as u8,
        ];

        let mut reader = U8Reader::new(&data);
        let decoder = <ARMv7 as Arch>::Decoder::default();

        match decoder.decode(&mut reader) {
            Ok(i) => format!("{:08X?} [{:08X?}] -> {i}", addr, instr,),
            Err(e) => format!("{e:?}"),
        }
    }

    pub(super) fn emulate_arm_instr(&mut self, bus: &mut Sysbus) {
        let instr = self.pipeline[0];

        #[cfg(feature = "decode")]
        {
            use std::io::Write;

            let reg = &self.regs;

            writeln!(
                self.decode_log,
                "{:<50}  ({:08X} {:08X} {:08X} {:08X} {:08X}) {}",
                Self::decode_arm_instr(self.regs.pc.wrapping_sub(4), instr),
                reg.get_reg_i(0),
                reg.get_reg_i(1),
                reg.get_reg_i(2),
                reg.get_reg_i(3),
                reg.get_reg_i(11),
                format!(
                    "N: {} Z: {} C: {} V {}",
                    reg.get_n() as u8,
                    reg.get_z() as u8,
                    reg.get_c() as u8,
                    reg.get_v() as u8
                )
            );
        }

        self.pipeline[0] = self.pipeline[1];
        self.regs.pc = self.regs.pc.wrapping_add(4);

        let cond = CONDITION_LUT[self.regs.get_flags() as usize | ((instr as usize >> 28) & 0xF)];

        if cond {
            ARM_LUT[((instr as usize) >> 16 & 0xFF0) | ((instr as usize) >> 4 & 0xF)](
                self, bus, instr,
            );
        } else {
            self.instruction_prefetch::<u32>(bus, MemoryAccess::S);
        }
    }

    // ARM.3: Branch and Exchange (BX)
    fn branch_and_exchange(&mut self, bus: &mut Sysbus, instr: u32) {
        self.instruction_prefetch::<u32>(bus, MemoryAccess::N);
        self.regs.pc = self.regs.get_reg_i(instr & 0xF);
        if self.regs.pc & 0x1 != 0 {
            self.regs.pc -= 1;
            self.regs.set_t(true);
            self.fill_thumb_instr_buffer(bus);
        } else {
            self.fill_arm_instr_buffer(bus)
        }
    }

    // ARM.4: Branch and Branch with Link (B, BL)
    fn branch_branch_with_link<const LINK: bool>(&mut self, bus: &mut Sysbus, instr: u32) {
        let offset = instr & 0xFF_FFFF;
        let offset = if (offset >> 23) == 1 {
            0xFF00_0000 | offset
        } else {
            offset
        };

        self.instruction_prefetch::<u32>(bus, MemoryAccess::N);
        if LINK {
            self.regs.set_reg(Reg::R14, self.regs.pc.wrapping_sub(4))
        }
        self.regs.pc = self.regs.pc.wrapping_add(offset << 2);
        self.fill_arm_instr_buffer(bus);
    }

    // ARM.5: Data Processing
    fn data_proc<
        const IMM: bool,
        const OPCODE: DataOp,
        const SET: bool,
        const SHIFT_TYPE: u32,
        const SHIFT_IMM: bool,
    >(
        &mut self,
        bus: &mut Sysbus,
        instr: u32,
    ) {
        use DataOp::*;

        let mut temp_inc_pc = false;

        let reg_dst = (instr >> 12) & 0xF;
        let reg_op1 = (instr >> 16) & 0xF;
        let reg_op2 = (instr >> 0) & 0xF;

        let (change_status, special_change_status) = if reg_dst == 15 && SET {
            (false, true)
        } else {
            (SET, false)
        };

        let op1 = self.regs.get_reg_i(reg_op1);
        let op2 = if IMM {
            let shift = (instr >> 8) & 0xF;
            let operand = instr & 0xFF;

            if ![Adc, Sbc, Rsc].contains(&OPCODE) && shift != 0 {
                self.shift(bus, 3, operand, shift * 2, true, change_status)
            } else {
                operand.rotate_right(shift * 2)
            }
        } else {
            let shift = if SHIFT_IMM {
                (instr >> 7) & 0x1F
            } else {
                // assert_eq!((instr >> 7) & 0x1, 0);
                let shift = self.regs.get_reg_i((instr >> 8) & 0xF) & 0xFF;
                self.regs.pc = self.regs.pc.wrapping_add(4); // Temp inc
                temp_inc_pc = true;
                shift
            };
            let op2 = self.regs.get_reg_i(reg_op2);
            // TODO: I Cycle occurs too early
            self.shift(
                bus,
                SHIFT_TYPE,
                op2,
                shift,
                SHIFT_IMM,
                change_status && ![Adc, Sbc, Rsc].contains(&OPCODE),
            )
        };

        let result = match OPCODE {
            And | Tst => op1 & op2,
            Eor | Teq => op1 ^ op2,
            Sub | Cmp => self.sub(op1, op2, change_status),
            Rsb => self.sub(op2, op1, change_status),
            Add | Cmn => self.add(op1, op2, change_status),
            Adc => self.adc(op1, op2, change_status),
            Sbc => self.sbc(op1, op2, change_status),
            Rsc => self.sbc(op2, op1, change_status),
            Orr => op1 | op2,
            Mov => op2,
            Bic => op1 & !op2,
            Mvn => !op2,
        };

        if change_status {
            self.regs.set_z(result == 0);
            self.regs.set_n(result & 0x8000_0000 != 0);
        } else if special_change_status {
            self.regs.set_reg(Reg::Cpsr, self.regs.get_reg(Reg::Spsr))
        } else {
            // assert!(OPCODE as u8 & 0xC != 0x8)
        }

        let mut clocked = false;
        if (OPCODE as u8) & 0xC != 0x8 {
            if reg_dst == 15 {
                clocked = true;
                self.instruction_prefetch::<u32>(bus, MemoryAccess::N);
                self.regs.pc = result;
                if self.regs.get_t() {
                    self.fill_thumb_instr_buffer(bus)
                } else {
                    self.fill_arm_instr_buffer(bus)
                }
            } else {
                self.regs.set_reg_i(reg_dst, result)
            }
        }
        if !clocked {
            if temp_inc_pc {
                self.regs.pc = self.regs.pc.wrapping_sub(4)
            } // Dec after temp inc
            self.instruction_prefetch::<u32>(bus, MemoryAccess::S);
        }
    }

    // ARM.6: PSR Transfer (MRS, MSR)
    fn psr_transfer<const IMM: bool, const P: bool, const L: bool>(
        &mut self,
        bus: &mut Sysbus,
        instr: u32,
    ) {
        // assert_eq!(instr >> 26 & 0b11, 0b00);
        // assert_eq!(instr >> 23 & 0b11, 0b10);
        let status_reg = if P { Reg::Spsr } else { Reg::Cpsr };
        let msr = L;
        // assert_eq!(instr >> 20 & 0b1, 0b0);
        self.instruction_prefetch::<u32>(bus, MemoryAccess::S);

        if msr {
            let mut mask = 0u32;
            if instr >> 19 & 0x1 != 0 {
                mask |= 0xFF000000
            } // Flags
            if instr >> 18 & 0x1 != 0 {
                mask |= 0x00FF0000
            } // Status
            if instr >> 17 & 0x1 != 0 {
                mask |= 0x0000FF00
            } // Extension
            if self.regs.get_mode() != Mode::User && instr >> 16 & 0x1 != 0 {
                mask |= 0x000000FF
            } // Control
              // assert_eq!(instr >> 12 & 0xF, 0xF);
            let operand = if IMM {
                let shift = instr >> 8 & 0xF;
                (instr & 0xFF).rotate_right(shift * 2)
            } else {
                // assert_eq!(instr >> 4 & 0xFF, 0);
                self.regs.get_reg_i(instr & 0xF)
            };
            let value = self.regs.get_reg(status_reg) & !mask | operand & mask;
            self.regs.set_reg(status_reg, value);
        } else {
            // assert!(!IMM);
            self.regs
                .set_reg_i(instr >> 12 & 0xF, self.regs.get_reg(status_reg));
            // assert_eq!(instr & 0xFFF, 0);
        }
    }

    // ARM.7: Multiply and Multiply-Accumulate (MUL, MLA)
    fn mul_mula<const ACCUMULATE: bool, const SET_STATUS: bool>(
        &mut self,
        bus: &mut Sysbus,
        instr: u32,
    ) {
        // assert_eq!(instr >> 22 & 0x3F, 0b000000);
        // assert_eq!(instr >> 4 & 0xF, 0b1001);

        let dest_reg = instr >> 16 & 0xF;
        let op1_reg = instr >> 12 & 0xF;
        let op1 = self.regs.get_reg_i(op1_reg);
        let op2 = self.regs.get_reg_i(instr >> 8 & 0xF);
        let op3 = self.regs.get_reg_i(instr & 0xF);

        self.instruction_prefetch::<u32>(bus, MemoryAccess::S);
        self.inc_mul_clocks(bus, op2, true);
        let result = if ACCUMULATE {
            self.internal(bus);
            op2.wrapping_mul(op3).wrapping_add(op1)
        } else {
            // assert_eq!(op1_reg, 0);
            op2.wrapping_mul(op3)
        };
        if SET_STATUS {
            self.regs.set_n(result & 0x8000_0000 != 0);
            self.regs.set_z(result == 0);
        }
        self.regs.set_reg_i(dest_reg, result);
    }

    // ARM.8: Multiply Long and Multiply-Accumulate Long (MULL, MLAL)
    fn mul_long<const SIGNED: bool, const ACCUMULATE: bool, const SET_STATUS: bool>(
        &mut self,
        bus: &mut Sysbus,
        instr: u32,
    ) {
        // assert_eq!(instr >> 23 & 0x1F, 0b00001);

        let src_dest_reg_high = instr >> 16 & 0xF;
        let src_dest_reg_low = instr >> 12 & 0xF;
        let op1 = self.regs.get_reg_i(instr >> 8 & 0xF);
        // assert_eq!(instr >> 4 & 0xF, 0b1001);
        let op2 = self.regs.get_reg_i(instr & 0xF);

        self.instruction_prefetch::<u32>(bus, MemoryAccess::S);
        self.internal(bus);
        self.inc_mul_clocks(bus, op1 as u32, SIGNED);
        let result = if SIGNED {
            (op1 as i32 as u64).wrapping_mul(op2 as i32 as u64)
        } else {
            (op1 as u64) * (op2 as u64)
        }
        .wrapping_add(if ACCUMULATE {
            self.internal(bus);
            (self.regs.get_reg_i(src_dest_reg_high) as u64) << 32
                | self.regs.get_reg_i(src_dest_reg_low) as u64
        } else {
            0
        });
        if SET_STATUS {
            self.regs.set_n(result & 0x8000_0000_0000_0000 != 0);
            self.regs.set_z(result == 0);
        }
        self.regs.set_reg_i(src_dest_reg_low, result as u32);
        self.regs
            .set_reg_i(src_dest_reg_high, (result >> 32) as u32);
    }

    // ARM.9: Single Data Transfer (LDR, STR)
    fn single_data_transfer<
        const SHIFTED_REG_OFFSET: bool,
        const PRE_OFFSET: bool,
        const ADD_OFFSET: bool,
        const TRANSFER_BYTE: bool,
        const WRITEBACK: bool,
        const L: bool,
    >(
        &mut self,
        bus: &mut Sysbus,
        instr: u32,
    ) {
        // assert_eq!(instr >> 26 & 0b11, 0b01);

        let mut write_back = WRITEBACK || !PRE_OFFSET;
        let load = instr >> 20 & 0x1 != 0;
        let base_reg = instr >> 16 & 0xF;
        let base = self.regs.get_reg_i(base_reg);
        let src_dest_reg = instr >> 12 & 0xF;
        self.instruction_prefetch::<u32>(bus, MemoryAccess::N);

        let offset = if SHIFTED_REG_OFFSET {
            let shift = instr >> 7 & 0x1F;
            let shift_type = instr >> 5 & 0x3;
            // assert_eq!(instr >> 4 & 0x1, 0);
            let offset_reg = instr & 0xF;
            // assert_ne!(offset_reg, 15);
            let operand = self.regs.get_reg_i(offset_reg);
            self.shift(bus, shift_type, operand, shift, true, false)
        } else {
            instr & 0xFFF
        };

        let mut exec = |addr| {
            if load {
                let access_type = if src_dest_reg == 15 {
                    MemoryAccess::N
                } else {
                    MemoryAccess::S
                };
                let value = if TRANSFER_BYTE {
                    self.read::<u8>(bus, access_type, addr) as u32
                } else {
                    self.read::<u32>(bus, access_type, addr & !0x3)
                        .rotate_right((addr & 0x3) * 8)
                };
                self.internal(bus);
                self.regs.set_reg_i(src_dest_reg, value);
                if src_dest_reg == base_reg {
                    write_back = false
                }
                if src_dest_reg == 15 {
                    self.fill_arm_instr_buffer(bus)
                }
            } else {
                let value = self.regs.get_reg_i(src_dest_reg);
                let value = if src_dest_reg == 15 {
                    value.wrapping_add(4)
                } else {
                    value
                };
                if TRANSFER_BYTE {
                    self.write::<u8>(bus, MemoryAccess::N, addr, value as u8);
                } else {
                    self.write::<u32>(bus, MemoryAccess::N, addr & !0x3, value);
                }
            }
        };
        let offset_applied = if ADD_OFFSET {
            base.wrapping_add(offset)
        } else {
            base.wrapping_sub(offset)
        };
        if PRE_OFFSET {
            exec(offset_applied);
            if write_back {
                self.regs.set_reg_i(base_reg, offset_applied)
            }
        } else {
            // TOOD: Take into account privilege of access
            // let force_non_privileged_access = instr >> 21 & 0x1 != 0;
            // assert!(!force_non_privileged_access);
            // Write back is not done if src_reg == base_reg
            exec(base);
            if write_back {
                self.regs.set_reg_i(base_reg, offset_applied)
            }
        }
    }

    // ARM.10: Halfword and Signed Data Transfer (STRH,LDRH,LDRSB,LDRSH)
    fn halfword_and_signed_data_transfer<
        const PRE_OFFSET: bool,
        const ADD_OFFSET: bool,
        const IMMEDIATE_OFFSET: bool,
        const WRITEBACK: bool,
        const LOAD: bool,
        const SIGNED: bool,
        const HALFWORD: bool,
    >(
        &mut self,
        bus: &mut Sysbus,
        instr: u32,
    ) {
        let mut write_back = WRITEBACK || !PRE_OFFSET;
        let base_reg = instr >> 16 & 0xF;
        let base = self.regs.get_reg_i(base_reg);
        let src_dest_reg = instr >> 12 & 0xF;
        let offset_hi = instr >> 8 & 0xF;
        // assert_eq!(instr >> 7 & 0x1, 1);
        let opcode = (SIGNED as u8) << 1 | (HALFWORD as u8);
        // assert_eq!(instr >> 4 & 0x1, 1);
        let offset_low = instr & 0xF;
        self.instruction_prefetch::<u32>(bus, MemoryAccess::N);

        let offset = if IMMEDIATE_OFFSET {
            offset_hi << 4 | offset_low
        } else {
            // assert_eq!(offset_hi, 0);
            self.regs.get_reg_i(offset_low)
        };

        let mut exec = |addr| {
            if LOAD {
                if src_dest_reg == base_reg {
                    write_back = false
                }
                let access_type = if src_dest_reg == 15 {
                    MemoryAccess::N
                } else {
                    MemoryAccess::S
                };
                // TODO: Make all access 16 bit
                let value = match opcode {
                    1 => (self.read::<u16>(bus, access_type, addr & !0x1) as u32)
                        .rotate_right((addr & 0x1) * 8),
                    2 => self.read::<u8>(bus, access_type, addr) as i8 as u32,
                    3 if addr & 0x1 == 1 => self.read::<u8>(bus, access_type, addr) as i8 as u32,
                    3 => self.read::<u16>(bus, access_type, addr) as i16 as u32,
                    _ => unreachable!(),
                };
                self.internal(bus);
                self.regs.set_reg_i(src_dest_reg, value);
                if src_dest_reg == 15 {
                    self.fill_arm_instr_buffer(bus)
                }
            } else {
                // assert_eq!(opcode, 1);
                let addr = addr & !0x1;
                let value = self.regs.get_reg_i(src_dest_reg);
                self.write::<u16>(bus, MemoryAccess::N, addr, value as u16);
            }
        };
        let offset_applied = if ADD_OFFSET {
            base.wrapping_add(offset)
        } else {
            base.wrapping_sub(offset)
        };
        if PRE_OFFSET {
            exec(offset_applied);
            if write_back {
                self.regs.set_reg_i(base_reg, offset_applied)
            }
        } else {
            exec(base);
            // assert!(instr >> 24 & 0x1 == 0);
            // Write back is not done if src_reg == base_reg
            if write_back {
                self.regs.set_reg_i(base_reg, offset_applied)
            }
        }
    }

    // ARM.11: Block Data Transfer (LDM,STM)
    fn block_data_transfer<
        const PRE_OFFSET: bool,    // P true
        const ADD_OFFSET: bool,    // U
        const PSR_FORCE_USR: bool, // S
        const WRITEBACK: bool,     // W true
        const LOAD: bool,          // L
    >(
        &mut self,
        bus: &mut Sysbus,
        instr: u32,
    ) {
        // assert_eq!(instr >> 25 & 0x7, 0b100);
        let pre_offset = PRE_OFFSET ^ !ADD_OFFSET;
        let base_reg = instr >> 16 & 0xF;
        // assert_ne!(base_reg, 0xF);
        let base = self.regs.get_reg_i(base_reg);
        let base_offset = base & 0x3;
        let base = base - base_offset;
        let mut r_list = (instr & 0xFFFF) as u16;
        let write_back = WRITEBACK && !(LOAD && r_list & (1 << base_reg) != 0);
        let actual_mode = self.regs.get_mode();
        if PSR_FORCE_USR && !(LOAD && r_list & 0x8000 != 0) {
            self.regs.set_mode(Mode::User)
        }

        self.instruction_prefetch::<u32>(bus, MemoryAccess::N);
        let mut loaded_pc = false;
        let num_regs = r_list.count_ones();
        let start_addr = if ADD_OFFSET {
            base
        } else {
            base.wrapping_sub(num_regs * 4)
        };
        let mut addr = start_addr;
        let final_addr = if ADD_OFFSET {
            addr + 4 * num_regs
        } else {
            start_addr
        } + base_offset;

        let (final_addr, inc_amount) = if num_regs == 0 {
            match ADD_OFFSET {
                true => (final_addr + 0x40, 0x40),
                false => (final_addr - 0x40, 0x40),
            }
        } else {
            (final_addr, 4)
        };

        let mut exec = |addr, reg, last_access| {
            if LOAD {
                let value = self.read::<u32>(bus, MemoryAccess::S, addr);
                self.regs.set_reg_i(reg, value);
                if write_back {
                    self.regs.set_reg_i(base_reg, final_addr)
                }
                if last_access {
                    self.internal(bus)
                }
                if reg == 15 {
                    if PSR_FORCE_USR {
                        self.regs.restore_cpsr()
                    }
                    loaded_pc = true;
                    self.next_access = MemoryAccess::N;
                    self.fill_arm_instr_buffer(bus);
                }
            } else {
                let value = self.regs.get_reg_i(reg);
                let access_type = if last_access {
                    MemoryAccess::N
                } else {
                    MemoryAccess::S
                };
                self.write::<u32>(
                    bus,
                    access_type,
                    addr,
                    if reg == 15 {
                        value.wrapping_add(4)
                    } else {
                        value
                    },
                );
                if write_back {
                    self.regs.set_reg_i(base_reg, final_addr)
                }
            }
        };
        if num_regs == 0 {
            let addr = match (ADD_OFFSET, PRE_OFFSET) {
                (true, true) => start_addr + 4,
                (true, false) => start_addr,
                (false, true) => start_addr - 0x40,
                (false, false) => start_addr - 0x3C,
            };

            exec(addr, 15, true);
        } else {
            let mut calc_addr = || {
                if pre_offset {
                    addr += inc_amount;
                    addr
                } else {
                    let old_addr = addr;
                    addr += inc_amount;
                    old_addr
                }
            };

            let mut reg = 0;
            while r_list != 0x1 {
                if r_list & 0x1 != 0 {
                    exec(calc_addr(), reg, false);
                }
                reg += 1;
                r_list >>= 1;
            }
            exec(calc_addr(), reg, true);
        }

        self.regs.set_mode(actual_mode);
    }

    // ARM.12: Single Data Swap (SWP)
    fn single_data_swap<const BYTE: bool>(&mut self, bus: &mut Sysbus, instr: u32) {
        // assert_eq!(instr >> 23 & 0x1F, 0b00010);
        // assert_eq!(instr >> 20 & 0x3, 0b00);
        // assert_eq!(instr >> 4 & 0xFF, 0b00001001);

        let base = self.regs.get_reg_i(instr >> 16 & 0xF);
        let dest_reg = instr >> 12 & 0xF;
        let src_reg = instr & 0xF;
        let src = self.regs.get_reg_i(src_reg);

        self.instruction_prefetch::<u32>(bus, MemoryAccess::N);
        let value = if BYTE {
            let value = self.read::<u8>(bus, MemoryAccess::N, base) as u32;
            self.write::<u8>(bus, MemoryAccess::S, base, src as u8);
            value
        } else {
            let value = self
                .read::<u32>(bus, MemoryAccess::N, base & !0x3)
                .rotate_right((base & 0x3) * 8);
            self.write::<u32>(bus, MemoryAccess::S, base & !0x3, src);
            value
        };
        self.regs.set_reg_i(dest_reg, value);
        self.internal(bus);
    }

    // ARM.13: Software Interrupt (SWI)
    fn arm_software_interrupt(&mut self, bus: &mut Sysbus, _instr: u32) {
        // assert_eq!(instr >> 24 & 0xF, 0b1111);
        self.instruction_prefetch::<u32>(bus, MemoryAccess::N);
        self.regs.change_mode(Mode::Supervisor);
        self.regs.set_reg(Reg::R14, self.regs.pc.wrapping_sub(4));
        self.regs.set_i(true);
        self.regs.pc = 0x8;
        self.fill_arm_instr_buffer(bus);
    }

    // ARM.14: Coprocessor Data Operations (CDP)
    // ARM.15: Coprocessor Data Transfers (LDC,STC)
    // ARM.16: Coprocessor Register Transfers (MRC, MCR)
    fn coprocessor(&mut self, _bus: &mut Sysbus, _instr: u32) {
        unimplemented!("Coprocessor not implemented!");
    }

    // ARM.17: Undefined Instruction
    #[allow(dead_code)]
    fn undefined_instr_arm(&mut self, _: &mut Sysbus, _: u32) {
        unimplemented!("ARM.17: Undefined Instruction not implemented!");
    }
}
