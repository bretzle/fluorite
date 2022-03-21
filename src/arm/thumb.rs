use super::Arm7tdmi;
use crate::arm::registers::Mode;
use crate::arm::registers::Reg;
use crate::arm::InstructionHandler;
use crate::arm::CONDITION_LUT;
use crate::io::{MemoryAccess, Sysbus};

include!(concat!(env!("OUT_DIR"), "/thumb_lut.rs"));

impl Arm7tdmi {
    pub(super) fn fill_thumb_instr_buffer(&mut self, bus: &mut Sysbus) {
        self.regs.pc &= !0x1;
        self.pipeline[0] = self.read::<u16>(bus, MemoryAccess::S, self.regs.pc & !0x1) as u32;
        self.regs.pc = self.regs.pc.wrapping_add(2);

        self.pipeline[1] = self.read::<u16>(bus, MemoryAccess::S, self.regs.pc & !0x1) as u32;
    }

    #[inline]
    #[cfg(feature = "decode")]
    pub fn decode_thumb_instr(addr: u32, instr: u16) -> String {
        use yaxpeax_arch::{Arch, Decoder, U8Reader};
        use yaxpeax_arm::armv7::ARMv7;

        let data = [(instr & 0xFF) as u8, (instr >> 8 & 0xFF) as u8];
        let mut reader = U8Reader::new(&data);
        let decoder = <ARMv7 as Arch>::Decoder::default_thumb();
        match decoder.decode(&mut reader) {
            Ok(i) => format!("{:08X?} [  {:04X?}  ] -> {i}", addr, instr,),
            Err(e) => format!("{e:?}"),
        }
    }

    pub(super) fn emulate_thumb_instr(&mut self, bus: &mut Sysbus) {
        let instr = self.pipeline[0] as u16;

        #[cfg(feature = "decode")]
        {
            use std::io::Write;

            let reg = &self.regs;

            writeln!(
                self.decode_log,
                "{:<50}  ({:08X} {:08X} {:08X} {:08X} {:08X}) {}",
                Self::decode_thumb_instr(self.regs.pc.wrapping_sub(2), instr),
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
        self.regs.pc = self.regs.pc.wrapping_add(2);

        THUMB_LUT[(instr >> 8) as usize](self, bus, instr);
    }

    // THUMB.1: move shifted register
    fn move_shifted_reg<const OpH: bool, const OpL: bool>(&mut self, bus: &mut Sysbus, instr: u16) {
        assert_eq!(instr >> 13, 0b000);
        let opcode = (OpH as u32) << 1 | (OpL as u32);
        let offset = (instr >> 6 & 0x1F) as u32;
        let src = self.regs.get_reg_i((instr >> 3 & 0x7) as u32);
        let dest_reg = (instr & 0x7) as u32;
        assert_ne!(opcode, 0b11);
        let result = self.shift(bus, opcode, src, offset, true, true);

        self.regs.set_n(result & 0x8000_0000 != 0);
        self.regs.set_z(result == 0);
        self.regs.set_reg_i(dest_reg, result);
        self.instruction_prefetch::<u16>(bus, MemoryAccess::S);
    }

    // THUMB.2: add/subtract
    fn add_sub<const IMM: bool, const SUB: bool>(&mut self, bus: &mut Sysbus, instr: u16) {
        assert_eq!(instr >> 11, 0b00011);

        let operand = (instr >> 6 & 0x7) as u32;
        let operand = if IMM {
            operand
        } else {
            self.regs.get_reg_i(operand)
        };
        let src = self.regs.get_reg_i((instr >> 3 & 0x7) as u32);
        let dest_reg = (instr & 0x7) as u32;

        let result = if SUB {
            self.sub(src, operand, true)
        } else {
            self.add(src, operand, true)
        };
        self.regs.set_reg_i(dest_reg, result);
        self.instruction_prefetch::<u16>(bus, MemoryAccess::S);
    }

    // THUMB.3: move/compare/add/subtract immediate
    fn immediate<
        const OpH: bool,
        const OpL: bool,
        const Rd2: bool,
        const Rd1: bool,
        const Rd0: bool,
    >(
        &mut self,
        bus: &mut Sysbus,
        instr: u16,
    ) {
        assert_eq!(instr >> 13, 0b001);

        let opcode = (OpH as u8) << 1 | (OpL as u8);
        let dest_reg = (Rd2 as u8) << 2 | (Rd1 as u8) << 1 | (Rd0 as u8);

        let immediate = (instr & 0xFF) as u32;
        let op1 = self.regs.get_reg_i(dest_reg as u32);
        let result = match opcode {
            0b00 => immediate,                      // MOV
            0b01 => self.sub(op1, immediate, true), // CMP
            0b10 => self.add(op1, immediate, true), // ADD
            0b11 => self.sub(op1, immediate, true), // SUB
            _ => unreachable!(),
        };
        self.regs.set_z(result == 0);
        self.regs.set_n(result & 0x8000_0000 != 0);

        if opcode != 0b01 {
            self.regs.set_reg_i(dest_reg as u32, result)
        }
        self.instruction_prefetch::<u16>(bus, MemoryAccess::S);
    }

    // THUMB.4: ALU operations
    fn alu(&mut self, bus: &mut Sysbus, instr: u16) {
        assert_eq!(instr >> 10 & 0x3F, 0b010000);

        self.instruction_prefetch::<u16>(bus, MemoryAccess::S);

        let opcode = instr >> 6 & 0xF;
        let src = self.regs.get_reg_i((instr >> 3 & 0x7) as u32);
        let dest_reg = (instr & 0x7) as u32;
        let dest = self.regs.get_reg_i(dest_reg);

        let result = match opcode {
            0x0 => dest & src,                                        // AND
            0x1 => dest ^ src,                                        // XOR
            0x2 => self.shift(bus, 0, dest, src & 0xFF, false, true), // LSL
            0x3 => self.shift(bus, 1, dest, src & 0xFF, false, true), // LSR
            0x4 => self.shift(bus, 2, dest, src & 0xFF, false, true), // ASR
            0x5 => self.adc(dest, src, true),                         // ADC
            0x6 => self.sbc(dest, src, true),                         // SBC
            0x7 => self.shift(bus, 3, dest, src & 0xFF, false, true), // ROR
            0x8 => dest & src,                                        // TST
            0x9 => self.sub(0, src, true),                            // NEG
            0xA => self.sub(dest, src, true),                         // CMP
            0xB => self.add(dest, src, true),                         // CMN
            0xC => dest | src,                                        // ORR
            0xD => {
                self.inc_mul_clocks(bus, dest, true);
                dest.wrapping_mul(src)
            } // MUL
            0xE => dest & !src,                                       // BIC
            0xF => !src,                                              // MVN
            _ => unreachable!(),
        };

        self.regs.set_n(result & 0x8000_0000 != 0);
        self.regs.set_z(result == 0);

        if ![0x8, 0xA, 0xB].contains(&opcode) {
            self.regs.set_reg_i(dest_reg, result)
        }
    }

    // THUMB.5: Hi register operations/branch exchange
    fn hi_reg_bx<const OpH: bool, const OpL: bool>(&mut self, bus: &mut Sysbus, instr: u16) {
        assert_eq!(instr >> 10, 0b010001);
        let opcode = (OpH as u16) << 1 | OpL as u16;
        let dest_reg_msb = instr >> 7 & 0x1;
        let src_reg_msb = instr >> 6 & 0x1;
        let src = self
            .regs
            .get_reg_i((src_reg_msb << 3 | instr >> 3 & 0x7) as u32);
        let dest_reg = (dest_reg_msb << 3 | instr & 0x7) as u32;
        let dest = self.regs.get_reg_i(dest_reg);
        let result = match opcode {
            0b00 => self.add(dest, src, false), // ADD
            0b01 => self.sub(dest, src, true),  // CMP
            0b10 => src,
            0b11 => {
                assert_eq!(dest_reg_msb, 0);
                self.instruction_prefetch::<u16>(bus, MemoryAccess::N);
                self.regs.pc = src;
                if src & 0x1 != 0 {
                    self.regs.pc = self.regs.pc & !0x1;
                    self.fill_thumb_instr_buffer(bus);
                } else {
                    self.regs.pc = self.regs.pc & !0x2;
                    self.regs.set_t(false);
                    self.fill_arm_instr_buffer(bus);
                }
                return;
            }
            _ => unreachable!(),
        };
        if opcode & 0x1 == 0 {
            self.regs.set_reg_i(dest_reg, result)
        }
        if dest_reg == 15 {
            self.instruction_prefetch::<u16>(bus, MemoryAccess::N);
            self.fill_thumb_instr_buffer(bus);
        } else {
            self.instruction_prefetch::<u16>(bus, MemoryAccess::S);
        }
    }

    // THUMB.6: load PC-relative
    fn load_pc_rel<const Rd2: bool, const Rd1: bool, const Rd0: bool>(
        &mut self,
        bus: &mut Sysbus,
        instr: u16,
    ) {
        assert_eq!(instr >> 11, 0b01001);
        let dest_reg = (Rd2 as u32) << 2 | (Rd1 as u32) << 1 | (Rd0 as u32);
        let offset = (instr & 0xFF) as u32;
        let addr = (self.regs.pc & !0x2).wrapping_add(offset * 4);
        self.instruction_prefetch::<u16>(bus, MemoryAccess::N);
        let value = self
            .read::<u32>(bus, MemoryAccess::N, addr & !0x3)
            .rotate_right((addr & 0x3) * 8);
        self.regs.set_reg_i(dest_reg, value);
        self.internal(bus);
    }

    // THUMB.7: load/store with register offset
    fn load_store_reg_offset<const OpH: bool, const OpL: bool>(
        &mut self,
        io: &mut Sysbus,
        instr: u16,
    ) {
        assert_eq!(instr >> 12, 0b0101);
        let opcode = (OpH as u8) << 1 | (OpL as u8);
        assert_eq!(instr >> 9 & 0x1, 0);
        let offset_reg = (instr >> 6 & 0x7) as u32;
        let base_reg = (instr >> 3 & 0x7) as u32;
        let addr = self
            .regs
            .get_reg_i(base_reg)
            .wrapping_add(self.regs.get_reg_i(offset_reg));
        let src_dest_reg = (instr & 0x7) as u32;
        self.instruction_prefetch::<u16>(io, MemoryAccess::N);
        if opcode & 0b10 != 0 {
            // Load
            let value = if opcode & 0b01 != 0 {
                self.read::<u8>(io, MemoryAccess::S, addr) as u32 // LDRB
            } else {
                self.read::<u32>(io, MemoryAccess::S, addr & !0x3)
                    .rotate_right((addr & 0x3) * 8) // LDR
            };
            self.regs.set_reg_i(src_dest_reg, value);
            self.internal(io);
        } else {
            // Store
            if opcode & 0b01 != 0 {
                // STRB
                self.write::<u8>(
                    io,
                    MemoryAccess::N,
                    addr,
                    self.regs.get_reg_i(src_dest_reg) as u8,
                );
            } else {
                // STR
                self.write::<u32>(
                    io,
                    MemoryAccess::N,
                    addr & !0x3,
                    self.regs.get_reg_i(src_dest_reg),
                );
            }
        }
    }

    // THUMB.8: load/store sign-extended byte/halfword
    fn load_store_sign_ext<const OpH: bool, const OpL: bool>(
        &mut self,
        io: &mut Sysbus,
        instr: u16,
    ) {
        assert_eq!(instr >> 12, 0b0101);

        let opcode = (OpH as u8) << 1 | (OpL as u8);

        assert_eq!(instr >> 9 & 0x1, 1);
        let offset_reg = (instr >> 6 & 0x7) as u32;
        let base_reg = (instr >> 3 & 0x7) as u32;
        let src_dest_reg = (instr & 0x7) as u32;
        let addr = self
            .regs
            .get_reg_i(base_reg)
            .wrapping_add(self.regs.get_reg_i(offset_reg));

        self.instruction_prefetch::<u16>(io, MemoryAccess::N);
        if opcode == 0 {
            // STRH
            self.write::<u16>(
                io,
                MemoryAccess::N,
                addr & !0x1,
                self.regs.get_reg_i(src_dest_reg) as u16,
            );
        } else {
            // Load
            // TODO: Is access width 1?
            let value = match opcode {
                1 => self.read::<u8>(io, MemoryAccess::S, addr) as i8 as u32,
                2 => (self.read::<u16>(io, MemoryAccess::S, addr & !0x1) as u32)
                    .rotate_right((addr & 0x1) * 8),
                3 if addr & 0x1 == 1 => self.read::<u8>(io, MemoryAccess::S, addr) as i8 as u32,
                3 => self.read::<u16>(io, MemoryAccess::S, addr) as i16 as u32,
                _ => unreachable!(),
            };
            self.regs.set_reg_i(src_dest_reg, value);
            self.internal(io);
        }
    }

    // THUMB.9: load/store with immediate offset
    fn load_store_imm_offset<const BYTE: bool, const LOAD: bool>(
        &mut self,
        bus: &mut Sysbus,
        instr: u16,
    ) {
        assert_eq!(instr >> 13, 0b011);

        let offset = (instr >> 6 & 0x1F) as u32;
        let base = self.regs.get_reg_i((instr >> 3 & 0x7) as u32);
        let src_dest_reg = (instr & 0x7) as u32;

        self.instruction_prefetch::<u16>(bus, MemoryAccess::N);
        if LOAD {
            // Is access width 1? Probably not, could be just bug in prev version
            let value = if BYTE {
                let addr = base.wrapping_add(offset);
                self.read::<u8>(bus, MemoryAccess::S, addr) as u32
            } else {
                let addr = base.wrapping_add(offset << 2);
                self.read::<u32>(bus, MemoryAccess::S, addr & !0x3)
                    .rotate_right((addr & 0x3) * 8)
            };
            self.regs.set_reg_i(src_dest_reg, value);
            self.internal(bus);
        } else {
            let value = self.regs.get_reg_i(src_dest_reg);
            // Is access width 1? Probably not, could be just bug in prev version
            if BYTE {
                self.write::<u8>(bus, MemoryAccess::N, base.wrapping_add(offset), value as u8);
            } else {
                self.write::<u32>(
                    bus,
                    MemoryAccess::N,
                    base.wrapping_add(offset << 2) & !0x3,
                    value,
                );
            }
        }
    }

    // THUMB.10: load/store halfword
    fn load_store_halfword<const LOAD: bool>(&mut self, bus: &mut Sysbus, instr: u16) {
        assert_eq!(instr >> 12, 0b1000);

        let offset = (instr >> 6 & 0x1F) as u32;
        let base = self.regs.get_reg_i((instr >> 3 & 0x7) as u32);
        let src_dest_reg = (instr & 0x7) as u32;
        let addr = base + offset * 2;

        self.instruction_prefetch::<u16>(bus, MemoryAccess::N);
        if LOAD {
            let value = (self.read::<u16>(bus, MemoryAccess::S, addr & !0x1) as u32)
                .rotate_right((addr & 0x1) * 8);
            self.regs.set_reg_i(src_dest_reg, value);
            self.internal(bus);
        } else {
            self.write::<u16>(
                bus,
                MemoryAccess::N,
                addr & !0x1,
                self.regs.get_reg_i(src_dest_reg) as u16,
            );
        }
    }

    // THUMB.11: load/store SP-relative
    fn load_store_sp_rel<const LOAD: bool, const Rd2: bool, const Rd1: bool, const Rd0: bool>(
        &mut self,
        bus: &mut Sysbus,
        instr: u16,
    ) {
        assert_eq!(instr >> 12 & 0xF, 0b1001);
        let src_dest_reg = (Rd2 as u32) << 2 | (Rd1 as u32) << 1 | (Rd0 as u32);
        let offset = (instr & 0xFF) * 4;
        let addr = self.regs.get_reg(Reg::R13).wrapping_add(offset as u32);
        self.instruction_prefetch::<u16>(bus, MemoryAccess::N);
        if LOAD {
            let value = self
                .read::<u32>(bus, MemoryAccess::S, addr & !0x3)
                .rotate_right((addr & 0x3) * 8);
            self.regs.set_reg_i(src_dest_reg, value);
            self.internal(bus);
        } else {
            self.write::<u32>(
                bus,
                MemoryAccess::N,
                addr & !0x3,
                self.regs.get_reg_i(src_dest_reg),
            );
        }
    }

    // THUMB.12: get relative address
    fn get_rel_addr<const SP: bool, const Rd2: bool, const Rd1: bool, const Rd0: bool>(
        &mut self,
        bus: &mut Sysbus,
        instr: u16,
    ) {
        assert_eq!(instr >> 12 & 0xF, 0b1010);

        let dest_reg = (Rd2 as u32) << 2 | (Rd1 as u32) << 1 | (Rd0 as u32);
        let src = if SP {
            // SP
            self.regs.get_reg(Reg::R13)
        } else {
            // PC
            self.regs.pc & !0x2
        };
        let offset = (instr & 0xFF) as u32;
        self.regs.set_reg_i(dest_reg, src.wrapping_add(offset * 4));
        self.instruction_prefetch::<u16>(bus, MemoryAccess::S);
    }

    // THUMB.13: add offset to stack pointer
    fn add_offset_sp(&mut self, bus: &mut Sysbus, instr: u16) {
        assert_eq!(instr >> 8 & 0xFF, 0b10110000);
        let sub = instr >> 7 & 0x1 != 0;
        let offset = ((instr & 0x7F) * 4) as u32;
        let sp = self.regs.get_reg(Reg::R13);
        let value = if sub {
            sp.wrapping_sub(offset)
        } else {
            sp.wrapping_add(offset)
        };
        self.regs.set_reg(Reg::R13, value);
        self.instruction_prefetch::<u16>(bus, MemoryAccess::S);
    }

    // THUMB.14: push/pop registers
    fn push_pop_regs<const POP: bool, const PC_LR: bool>(&mut self, io: &mut Sysbus, instr: u16) {
        assert_eq!(instr >> 12 & 0xF, 0b1011);
        assert_eq!(instr >> 9 & 0x3, 0b10);

        let mut r_list = (instr & 0xFF) as u8;
        self.instruction_prefetch::<u16>(io, MemoryAccess::N);
        if POP {
            let mut sp = self.regs.get_reg(Reg::R13);
            let mut stack_pop = |sp, last_access, reg: u32| {
                let value = self.read::<u32>(io, MemoryAccess::S, sp);
                self.regs.set_reg_i(reg, value);
                if last_access {
                    self.internal(io)
                }
            };
            let mut reg = 0;
            while r_list != 0 {
                if r_list & 0x1 != 0 {
                    stack_pop(sp, r_list == 1 && !PC_LR, reg);
                    sp += 4;
                }
                reg += 1;
                r_list >>= 1;
            }
            if PC_LR {
                stack_pop(sp, true, 15);
                self.regs.pc &= !0x1;
                sp += 4;
                // TODO: Verify
                self.next_access = MemoryAccess::N;
                self.fill_thumb_instr_buffer(io);
            }
            self.regs.set_reg(Reg::R13, sp);
        } else {
            let initial_sp = self.regs.get_reg(Reg::R13);
            let mut sp = self
                .regs
                .get_reg(Reg::R13)
                .wrapping_sub(4 * (r_list.count_ones() + PC_LR as u32));
            self.regs.set_reg(Reg::R13, sp);
            let regs_copy = self.regs.clone();
            let mut stack_push = |sp, value, last_access| {
                self.write::<u32>(io, MemoryAccess::S, sp, value);
                if last_access {
                    self.next_access = MemoryAccess::N
                }
            };
            let mut reg = 0;
            while r_list != 0 {
                if r_list & 0x1 != 0 {
                    stack_push(sp, regs_copy.get_reg_i(reg), r_list == 0x1 && !PC_LR);
                    sp += 4;
                }
                reg += 1;
                r_list >>= 1;
            }
            if PC_LR {
                stack_push(sp, regs_copy.get_reg(Reg::R14), true);
                sp += 4
            }
            assert_eq!(initial_sp, sp);
        }
    }

    // THUMB.15: multiple load/store
    fn multiple_load_store<const LOAD: bool, const Rb2: bool, const Rb1: bool, const Rb0: bool>(
        &mut self,
        io: &mut Sysbus,
        instr: u16,
    ) {
        assert_eq!(instr >> 12, 0b1100);
        let base_reg = (Rb2 as u32) << 2 | (Rb1 as u32) << 1 | (Rb0 as u32);
        let mut base = self.regs.get_reg_i(base_reg);
        let base_offset = base & 0x3;
        base -= base_offset;
        let mut r_list = (instr & 0xFF) as u8;

        self.instruction_prefetch::<u16>(io, MemoryAccess::N);
        let mut reg = 0;
        let mut first = true;
        let final_base = base.wrapping_add(4 * r_list.count_ones()) + base_offset;
        if !LOAD {
            self.regs.pc = self.regs.pc.wrapping_add(2);
        }
        let mut exec = |reg, last_access| {
            let addr = base;
            base = base.wrapping_add(4);
            if LOAD {
                let value = self.read::<u32>(io, MemoryAccess::S, addr);
                self.regs.set_reg_i(reg, value);
                if last_access {
                    self.internal(io)
                }
            } else {
                self.write::<u32>(io, MemoryAccess::S, addr, self.regs.get_reg_i(reg));
                if last_access {
                    self.next_access = MemoryAccess::N
                }
                if first {
                    self.regs.set_reg_i(base_reg, final_base);
                    first = false
                }
            }
        };
        if r_list == 0 {
            exec(15, true);
            if LOAD {
                self.fill_thumb_instr_buffer(io);
            }
            base = base.wrapping_add(0x3C + base_offset);
        } else {
            while r_list != 0x1 {
                if r_list & 0x1 != 0 {
                    exec(reg, false);
                }
                reg += 1;
                r_list >>= 1;
            }
            exec(reg, true);
        }
        //if load { io.inc_clock(Cycle::S, self.regs.pc.wrapping_add(2), 1) }
        if !LOAD {
            self.regs.pc = self.regs.pc.wrapping_sub(2)
        }
        self.regs.set_reg_i(base_reg, base + base_offset);
    }

    // THUMB.16: conditional branch
    fn cond_branch<const C3: bool, const C2: bool, const C1: bool, const C0: bool>(
        &mut self,
        bus: &mut Sysbus,
        instr: u16,
    ) {
        assert_eq!(instr >> 12, 0b1101);
        let condition =
            (C3 as usize) << 3 | (C2 as usize) << 2 | (C1 as usize) << 1 | (C0 as usize);
        assert_eq!(condition < 0xE, true);
        let offset = (instr & 0xFF) as i8 as u32;
        // if self.should_exec(condition as u32) {
        if CONDITION_LUT[self.regs.get_flags() as usize | condition] {
            self.instruction_prefetch::<u16>(bus, MemoryAccess::N);
            self.regs.pc = self.regs.pc.wrapping_add(offset.wrapping_mul(2));
            self.fill_thumb_instr_buffer(bus);
        } else {
            self.instruction_prefetch::<u16>(bus, MemoryAccess::S);
        }
    }

    // THUMB.17: software interrupt
    fn thumb_software_interrupt(&mut self, io: &mut Sysbus, instr: u16) {
        assert_eq!(instr >> 8 & 0xFF, 0b11011111);
        self.instruction_prefetch::<u16>(io, MemoryAccess::N);
        self.regs.change_mode(Mode::SVC);
        self.regs.set_reg(Reg::R14, self.regs.pc.wrapping_sub(2));
        self.regs.set_t(false);
        self.regs.set_i(true);
        self.regs.pc = 0x8;
        self.fill_arm_instr_buffer(io);
    }

    // THUMB.18: unconditional branch
    fn uncond_branch(&mut self, io: &mut Sysbus, instr: u16) {
        assert_eq!(instr >> 11, 0b11100);
        let offset = (instr & 0x7FF) as u32;
        let offset = if offset >> 10 & 0x1 != 0 {
            0xFFFF_F800 | offset
        } else {
            offset
        };

        self.instruction_prefetch::<u16>(io, MemoryAccess::N);
        self.regs.pc = self.regs.pc.wrapping_add(offset << 1);
        self.fill_thumb_instr_buffer(io);
    }

    // THUMB.19: long branch with link
    fn branch_with_link<const H: bool>(&mut self, bus: &mut Sysbus, instr: u16) {
        assert_eq!(instr >> 12, 0xF);

        let offset = (instr & 0x7FF) as u32;
        if H {
            // Second Instruction
            self.instruction_prefetch::<u16>(bus, MemoryAccess::N);
            let next_instr_pc = self.regs.pc.wrapping_sub(2);
            self.regs.pc = self.regs.get_reg(Reg::R14).wrapping_add(offset << 1);
            self.regs.set_reg(Reg::R14, next_instr_pc | 0x1);
            self.fill_thumb_instr_buffer(bus);
        } else {
            // First Instruction
            let offset = if offset >> 10 & 0x1 != 0 {
                0xFFFF_F800 | offset
            } else {
                offset
            };
            assert_eq!(instr >> 11, 0b11110);
            self.regs
                .set_reg(Reg::R14, self.regs.pc.wrapping_add(offset << 12));
            self.instruction_prefetch::<u16>(bus, MemoryAccess::S);
        }
    }

    fn undefined_instr_thumb(&mut self, _: &mut Sysbus, _: u16) {
        panic!("Undefined Thumb Instruction!")
    }
}
