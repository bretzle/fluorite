use std::fmt;

use fluorite_common::{bitfield, traits::UnsafeFrom};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Reg {
    R0 = 0,
    R1 = 1,
    R2 = 2,
    R3 = 3,
    R4 = 4,
    R5 = 5,
    R6 = 6,
    R7 = 7,
    R8 = 8,
    R9 = 9,
    R10 = 10,
    R11 = 11,
    R12 = 12,
    R13 = 13, // SP
    R14 = 14, // LR
    R15 = 15, // PC
    Cpsr,
    Spsr,
}

#[derive(Clone)]
pub struct Registers {
    usr: [u32; 15],
    fiq: [u32; 7],
    svc: [u32; 2],
    abt: [u32; 2],
    irq: [u32; 2],
    und: [u32; 2],
    pub pc: u32,
    cpsr: StatusRegister,
    spsr: [StatusRegister; 5],
}

impl Registers {
    pub fn new() -> Self {
        let mut ret = Self {
            usr: [0; 15],
            fiq: [0; 7],
            abt: [0; 2],
            svc: [0; 2],
            irq: [0; 2],
            und: [0; 2],
            pc: 0,
            cpsr: StatusRegister::reset(),
            spsr: [StatusRegister::reset(); 5],
        };

        ret.usr[13] = 0x03007F00;
        ret.irq[0] = 0x03007FA0;
        ret.svc[0] = 0x03007FE0;
        ret
    }

    pub fn skip_bios(&mut self) {
        self.pc = 0x08000000;
        self.usr[13] = 0x0300_7F00;
        self.cpsr.0 = 0x5F;
    }

    pub fn get_reg(&self, reg: Reg) -> u32 {
        let mode = self.cpsr.mode();
        use Reg::*;
        match reg {
            R0 | R1 | R2 | R3 | R4 | R5 | R6 | R7 => self.usr[reg as usize],
            R8 | R9 | R10 | R11 | R12 => match mode {
                Mode::Fiq => self.fiq[reg as usize - 8],
                _ => self.usr[reg as usize],
            },
            R13 | R14 => match mode {
                Mode::Fiq => self.fiq[reg as usize - 8],
                Mode::Supervisor => self.svc[reg as usize - 13],
                Mode::Abort => self.abt[reg as usize - 13],
                Mode::Irq => self.irq[reg as usize - 13],
                Mode::Undefined => self.und[reg as usize - 13],
                _ => self.usr[reg as usize],
            },
            R15 => self.pc,
            Cpsr => self.cpsr.raw(),
            Spsr => match mode {
                Mode::Fiq => self.spsr[0].raw(),
                Mode::Supervisor => self.spsr[1].raw(),
                Mode::Abort => self.spsr[2].raw(),
                Mode::Irq => self.spsr[3].raw(),
                Mode::Undefined => self.spsr[4].raw(),
                _ => self.cpsr.raw(),
            },
        }
    }

    pub fn set_reg(&mut self, reg: Reg, value: u32) {
        let mode = self.cpsr.mode();
        use Reg::*;
        match reg {
            R0 | R1 | R2 | R3 | R4 | R5 | R6 | R7 => self.usr[reg as usize] = value,
            R8 | R9 | R10 | R11 | R12 => match mode {
                Mode::Fiq => self.fiq[reg as usize - 8] = value,
                _ => self.usr[reg as usize] = value,
            },
            R13 | R14 => match mode {
                Mode::Fiq => self.fiq[reg as usize - 8] = value,
                Mode::Supervisor => self.svc[reg as usize - 13] = value,
                Mode::Abort => self.abt[reg as usize - 13] = value,
                Mode::Irq => self.irq[reg as usize - 13] = value,
                Mode::Undefined => self.und[reg as usize - 13] = value,
                _ => self.usr[reg as usize] = value,
            },
            R15 => self.pc = value,
            Cpsr => self.cpsr.0 = value,
            Spsr => match mode {
                Mode::Fiq => self.spsr[0] = StatusRegister(value),
                Mode::Supervisor => self.spsr[1] = StatusRegister(value),
                Mode::Abort => self.spsr[2] = StatusRegister(value),
                Mode::Irq => self.spsr[3] = StatusRegister(value),
                Mode::Undefined => self.spsr[4] = StatusRegister(value),
                _ => (),
            },
        }
    }

    fn get_reg_from_u32(&self, reg: u32) -> Reg {
        use Reg::*;
        match reg {
            0 => R0,
            1 => R1,
            2 => R2,
            3 => R3,
            4 => R4,
            5 => R5,
            6 => R6,
            7 => R7,
            8 => R8,
            9 => R9,
            10 => R10,
            11 => R11,
            12 => R12,
            13 => R13,
            14 => R14,
            15 => R15,
            _ => unreachable!(),
        }
    }

    pub fn restore_cpsr(&mut self) {
        self.cpsr.0 = self.get_reg(Reg::Spsr);
    }

    pub fn change_mode(&mut self, mode: Mode) {
        let cpsr = self.get_reg(Reg::Cpsr);
        self.set_mode(mode);
        self.set_reg(Reg::Spsr, cpsr);
    }

    pub fn get_reg_i(&self, reg: u32) -> u32 {
        self.get_reg(self.get_reg_from_u32(reg))
    }

    pub fn set_reg_i(&mut self, reg: u32, value: u32) {
        self.set_reg(self.get_reg_from_u32(reg), value);
    }

    pub fn get_status(&self) -> StatusRegister {
        self.cpsr
    }

    pub fn get_n(&self) -> bool {
        self.cpsr.negative()
    }
    pub fn get_z(&self) -> bool {
        self.cpsr.zero()
    }
    pub fn get_c(&self) -> bool {
        self.cpsr.carry()
    }
    pub fn get_v(&self) -> bool {
        self.cpsr.overflow()
    }
    pub fn get_i(&self) -> bool {
        self.cpsr.irq_disabled()
    }
    pub fn _get_f(&self) -> bool {
        self.cpsr.fiq_disabled()
    }
    pub fn get_flags(&self) -> u32 {
        self.cpsr.raw() >> 24
    }
    pub fn get_t(&self) -> bool {
        self.cpsr.thumb_state()
    }
    pub fn get_mode(&self) -> Mode {
        self.cpsr.mode()
    }
    pub fn set_n(&mut self, value: bool) {
        self.cpsr.set_negative(value)
    }
    pub fn set_z(&mut self, value: bool) {
        self.cpsr.set_zero(value)
    }
    pub fn set_c(&mut self, value: bool) {
        self.cpsr.set_carry(value)
    }
    pub fn set_v(&mut self, value: bool) {
        self.cpsr.set_overflow(value)
    }
    pub fn set_i(&mut self, value: bool) {
        self.cpsr.set_irq_disabled(value)
    }
    pub fn _set_f(&mut self, value: bool) {
        self.cpsr.set_fiq_disabled(value)
    }
    pub fn set_t(&mut self, value: bool) {
        self.cpsr.set_thumb_state(value)
    }
    pub fn set_mode(&mut self, mode: Mode) {
        self.cpsr.set_mode(mode)
    }
}

#[repr(u8)]
#[derive(Debug, PartialEq)]
pub enum Mode {
    User = 0b10000,
    Fiq = 0b10001,
    Irq = 0b10010,
    Supervisor = 0b10011,
    Abort = 0b10111,
    System = 0b11111,
    Undefined = 0b11011,
}

impl UnsafeFrom<u8> for Mode {
    #[inline]
    unsafe fn from(raw: u8) -> Self {
        core::mem::transmute(raw)
    }
}

impl From<Mode> for u8 {
    #[inline]
    fn from(mode: Mode) -> Self {
        mode as u8
    }
}

bitfield! {
    #[derive(Clone, Copy)]
    pub struct StatusRegister(u32) {
        pub raw: u32 [read_only] @ ..,
        pub mode: u8 [unsafe Mode] @ 0..=4,
        pub thumb_state: bool @ 5,
        pub fiq_disabled: bool @ 6,
        pub irq_disabled: bool @ 7,
        pub overflow: bool @ 28,
        pub carry: bool @ 29,
        pub zero: bool @ 30,
        pub negative: bool @ 31,
    }
}

impl StatusRegister {
    pub const fn reset() -> Self {
        Self(Mode::System as u32)
    }
}

impl fmt::Display for StatusRegister {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{N}{Z}{C}{V}{I}{F}{T}] {MODE:?}",
            N = if self.negative() { 'N' } else { '-' },
            Z = if self.zero() { 'Z' } else { '-' },
            C = if self.carry() { 'C' } else { '-' },
            V = if self.overflow() { 'V' } else { '-' },
            I = if self.irq_disabled() { 'I' } else { '-' },
            F = if self.fiq_disabled() { 'F' } else { '-' },
            T = if self.thumb_state() { 'T' } else { '-' },
            MODE = self.mode()
        )
    }
}
