#[allow(clippy::module_inception)]
mod arm;
pub mod registers;
mod thumb;

use self::registers::{Mode, Reg, Registers};
use crate::io::{memory::MemoryValue, Cycle, MemoryAccess, Sysbus};
use num::cast;
use std::mem::size_of;

include!(concat!(env!("OUT_DIR"), "/cond_lut.rs"));

pub(self) type InstructionHandler<T> = fn(&mut Arm7tdmi, &mut Sysbus, T);

pub struct Arm7tdmi {
    pub regs: Registers,
    pipeline: [u32; 2],
    next_access: MemoryAccess,
    internal: bool,

    #[cfg(feature = "decode")]
    decode_log: std::fs::File,
}

impl Arm7tdmi {
    pub fn new(skip_bios: bool, bus: &mut Sysbus) -> Self {
        let mut arm = Self {
            regs: Registers::new(),
            pipeline: [0; 2],
            next_access: MemoryAccess::N,
            internal: false,

            #[cfg(feature = "decode")]
            decode_log: std::fs::File::create("decode.log").unwrap(),
        };

        if skip_bios {
            arm.regs.skip_bios();
        }

        arm.fill_arm_instr_buffer(bus);
        arm
    }

    pub fn reset(&mut self, skip_bios: bool, bus: &mut Sysbus) {
        self.regs = Registers::new();
        self.pipeline = [0; 2];
        self.next_access = MemoryAccess::N;
        self.internal = false;
        if skip_bios {
            self.regs.skip_bios();
        }
        self.fill_arm_instr_buffer(bus);
    }

    pub fn emulate_instr(&mut self, bus: &mut Sysbus) {
        if self.regs.get_t() {
            self.emulate_thumb_instr(bus);
        } else {
            self.emulate_arm_instr(bus);
        }
    }

    pub fn read<T>(&mut self, bus: &mut Sysbus, access: MemoryAccess, addr: u32) -> T
    where
        T: MemoryValue,
    {
        bus.setup_openbus(self.regs.pc, self.regs.get_t(), &self.pipeline);
        let val = bus.read(addr);
        bus.inc_clock(
            self.next_access,
            addr,
            match size_of::<T>() {
                1 => 0,
                2 => 1,
                4 => 2,
                _ => unreachable!(),
            },
        );
        self.next_access = access;
        val
    }

    pub fn write<T>(&mut self, bus: &mut Sysbus, access: MemoryAccess, addr: u32, value: T)
    where
        T: MemoryValue,
    {
        bus.setup_openbus(self.regs.pc, self.regs.get_t(), &self.pipeline);
        bus.inc_clock(
            self.next_access,
            addr,
            match size_of::<T>() {
                1 => 0,
                2 => 1,
                4 => 2,
                _ => unreachable!(),
            },
        );
        self.next_access = access;
        bus.write(addr, value);
    }

    pub fn instruction_prefetch<T>(&mut self, bus: &mut Sysbus, access: MemoryAccess)
    where
        T: MemoryValue,
    {
        self.pipeline[1] = cast::<T, u32>(self.read(bus, access, self.regs.pc)).unwrap();
        self.internal = false;
    }

    pub fn internal(&mut self, bus: &mut Sysbus) {
        bus.setup_openbus(self.regs.pc, self.regs.get_t(), &self.pipeline);
        bus.inc_clock(Cycle::I, 0, 0);
        self.next_access = MemoryAccess::N;
    }

    pub fn handle_irq(&mut self, bus: &mut Sysbus) {
        if self.regs.get_i() || !bus.interrupts_requested() {
            return;
        }
        self.regs.change_mode(Mode::Irq);
        let lr = if self.regs.get_t() {
            self.read::<u16>(bus, MemoryAccess::N, self.regs.pc);
            self.regs.pc.wrapping_sub(2).wrapping_add(4)
        } else {
            self.read::<u32>(bus, MemoryAccess::N, self.regs.pc);
            self.regs.pc.wrapping_sub(4).wrapping_add(4)
        };
        self.regs.set_reg(Reg::R14, lr);
        self.regs.set_t(false);
        self.regs.set_i(true);
        self.regs.pc = 0x18;
        self.fill_arm_instr_buffer(bus);
    }
}

impl Arm7tdmi {
    pub(self) fn shift(
        &mut self,
        bus: &mut Sysbus,
        shift_type: u32,
        operand: u32,
        shift: u32,
        immediate: bool,
        change_status: bool,
    ) -> u32 {
        if immediate && shift == 0 {
            match shift_type {
                // LSL #0
                0 => operand,
                // LSR #32
                1 => {
                    if change_status {
                        self.regs.set_c(operand >> 31 != 0)
                    }
                    0
                }
                // ASR #32
                2 => {
                    let bit = operand >> 31 != 0;
                    if change_status {
                        self.regs.set_c(bit);
                    }
                    if bit {
                        0xFFFF_FFFF
                    } else {
                        0
                    }
                }
                // RRX #1
                3 => {
                    let new_c = operand & 0x1 != 0;
                    let op2 = (self.regs.get_c() as u32) << 31 | operand >> 1;
                    if change_status {
                        self.regs.set_c(new_c)
                    }
                    op2
                }
                _ => unreachable!(),
            }
        } else if shift > 31 {
            assert!(!immediate);
            if !immediate {
                self.internal(bus)
            }
            match shift_type {
                // LSL
                0 => {
                    if change_status {
                        if shift == 32 {
                            self.regs.set_c(operand << (shift - 1) & 0x8000_0000 != 0)
                        } else {
                            self.regs.set_c(false)
                        }
                    }
                    0
                }
                // LSR
                1 => {
                    if change_status {
                        if shift == 32 {
                            self.regs.set_c(operand >> (shift - 1) & 0x1 != 0)
                        } else {
                            self.regs.set_c(false)
                        }
                    }
                    0
                }
                // ASR
                2 => {
                    let c = operand & 0x8000_0000 != 0;
                    if change_status {
                        self.regs.set_c(c)
                    }
                    if c {
                        0xFFFF_FFFF
                    } else {
                        0
                    }
                }
                // ROR
                3 => {
                    let shift = shift & 0x1F;
                    let shift = if shift == 0 { 0x20 } else { shift };
                    if change_status {
                        self.regs.set_c(operand >> (shift - 1) & 0x1 != 0)
                    }
                    operand.rotate_right(shift)
                }
                _ => unreachable!(),
            }
        } else {
            if !immediate {
                self.internal(bus)
            }
            let change_status = change_status && shift != 0;
            match shift_type {
                // LSL
                0 => {
                    if change_status {
                        self.regs.set_c(operand << (shift - 1) & 0x8000_0000 != 0);
                    }
                    operand << shift
                }
                // LSR
                1 => {
                    if change_status {
                        self.regs.set_c(operand >> (shift - 1) & 0x1 != 0);
                    }
                    operand >> shift
                }
                // ASR
                2 => {
                    if change_status {
                        self.regs.set_c((operand as i32) >> (shift - 1) & 0x1 != 0)
                    };
                    ((operand as i32) >> shift) as u32
                }
                // ROR
                3 => {
                    if change_status {
                        self.regs.set_c(operand >> (shift - 1) & 0x1 != 0);
                    }
                    operand.rotate_right(shift)
                }
                _ => unreachable!(),
            }
        }
    }

    pub(self) fn add(&mut self, op1: u32, op2: u32, change_status: bool) -> u32 {
        let result = op1.overflowing_add(op2);
        if change_status {
            self.regs.set_c(result.1);
            self.regs.set_v((op1 as i32).overflowing_add(op2 as i32).1);
            self.regs.set_z(result.0 == 0);
            self.regs.set_n(result.0 & 0x8000_0000 != 0);
        }
        result.0
    }

    pub(self) fn adc(&mut self, op1: u32, op2: u32, change_status: bool) -> u32 {
        let result = op1.overflowing_add(op2);
        let result2 = result.0.overflowing_add(self.regs.get_c() as u32);
        if change_status {
            self.regs.set_c(result.1 || result2.1);
            self.regs.set_z(result2.0 == 0);
            self.regs.set_n(result2.0 & 0x8000_0000 != 0);
            self.regs
                .set_v((!(op1 ^ op2)) & (op1 ^ result2.0) & 0x8000_0000 != 0);
        }
        result2.0 as u32
    }

    pub(self) fn sub(&mut self, op1: u32, op2: u32, change_status: bool) -> u32 {
        let old_c = self.regs.get_c();
        self.regs.set_c(true);
        let result = self.adc(op1, !op2, change_status); // Simulate adding op1 + !op2 + 1
        if !change_status {
            self.regs.set_c(old_c)
        }
        result
    }

    pub(self) fn sbc(&mut self, op1: u32, op2: u32, change_status: bool) -> u32 {
        self.adc(op1, !op2, change_status)
    }

    pub(self) fn inc_mul_clocks(&mut self, bus: &mut Sysbus, op1: u32, signed: bool) {
        let mut mask = 0xFFFF_FF00;
        loop {
            self.internal(bus);
            let value = op1 & mask;
            if mask == 0 || value == 0 || signed && value == mask {
                break;
            }
            mask <<= 8;
        }
    }
}

#[derive(PartialEq, Eq)]
pub enum DataOp {
    And,
    Eor,
    Sub,
    Rsb,
    Add,
    Adc,
    Sbc,
    Rsc,
    Tst,
    Teq,
    Cmp,
    Cmn,
    Orr,
    Mov,
    Bic,
    Mvn,
}
