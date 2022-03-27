use std::fmt;

include!(concat!(env!("OUT_DIR"), "/cond_lut.rs"));

pub struct Registers {
    usr: [u32; 15],
    fiq: [u32; 7],
    svc: [u32; 2],
    abt: [u32; 2],
    irq: [u32; 2],
    und: [u32; 2],
    pub pc: u32,
    pub cpsr: StatusRegister,
    spsr: [StatusRegister; 5],
}

impl Registers {
    pub fn new() -> Self {
        Self {
            usr: [0; 15],
            fiq: [0; 7],
            abt: [0; 2],
            svc: [0; 2],
            irq: [0; 2],
            und: [0; 2],
            pc: 0,
            cpsr: StatusRegister::new(),
            spsr: [StatusRegister::new(); 5],
        }
    }

    pub(crate) fn switch_mode() {
        todo!()
    }

    fn mode_to_bank() {
        todo!()
    }

    pub fn check(&self, instr: u32) -> bool {
        CONDITION_LUT[self.cpsr.as_u32() as usize | ((instr as usize >> 28) & 0xF)]
    }

	pub fn get_reg(&self, reg: Reg) -> u32 {
        let mode = self.cpsr.mode;
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
            Cpsr => self.cpsr.as_u32(),
            Spsr => match mode {
                Mode::Fiq => self.spsr[0].as_u32(),
                Mode::Supervisor => self.spsr[1].as_u32(),
                Mode::Abort => self.spsr[2].as_u32(),
                Mode::Irq => self.spsr[3].as_u32(),
                Mode::Undefined => self.spsr[4].as_u32(),
                _ => self.cpsr.as_u32(),
            },
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum Mode {
    User = 0b10000,
    Fiq = 0b10001,
    Irq = 0b10010,
    Supervisor = 0b10011,
    Abort = 0b10111,
    System = 0b11111,
    Undefined = 0b11011,
}

#[derive(Clone, Copy)]
pub struct StatusRegister {
    mode: Mode,
    t: bool,
    f: bool,
    i: bool,
    v: bool,
    c: bool,
    z: bool,
    n: bool,
}

impl fmt::Display for StatusRegister {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{N}{Z}{C}{V}{I}{F}{T}] {MODE:?}",
            N = if self.n { 'N' } else { '-' },
            Z = if self.z { 'Z' } else { '-' },
            C = if self.c { 'C' } else { '-' },
            V = if self.v { 'V' } else { '-' },
            I = if self.i { 'I' } else { '-' },
            F = if self.f { 'F' } else { '-' },
            T = if self.t { 'T' } else { '-' },
            MODE = self.mode
        )
    }
}

impl StatusRegister {
    pub fn new() -> Self {
        Self {
            mode: Mode::Supervisor,
            t: false,
            f: false,
            i: false,
            v: false,
            c: false,
            z: false,
            n: false,
        }
    }

    pub fn set(&mut self, val: u32) {
        self.mode = match val & 0x1F {
            m if m == Mode::User as u32 => Mode::User,
            m if m == Mode::Fiq as u32 => Mode::Fiq,
            m if m == Mode::Irq as u32 => Mode::Irq,
            m if m == Mode::Supervisor as u32 => Mode::Supervisor,
            m if m == Mode::Abort as u32 => Mode::Abort,
            m if m == Mode::System as u32 => Mode::System,
            m if m == Mode::Undefined as u32 => Mode::Undefined,
            bits => panic!("Invalid Mode: {bits:05b}"),
        };
        self.t = ((val >> 5) & 0b1) != 0;
        self.f = ((val >> 6) & 0b1) != 0;
        self.i = ((val >> 7) & 0b1) != 0;
        self.v = ((val >> 28) & 0b1) != 0;
        self.c = ((val >> 29) & 0b1) != 0;
        self.z = ((val >> 30) & 0b1) != 0;
        self.n = ((val >> 31) & 0b1) != 0;
    }

    pub fn as_u32(&self) -> u32 {
        (self.mode as u32)
            | ((self.t as u32) << 5)
            | ((self.f as u32) << 6)
            | ((self.i as u32) << 7)
            | ((self.v as u32) << 28)
            | ((self.c as u32) << 29)
            | ((self.z as u32) << 30)
            | ((self.n as u32) << 31)
    }

    pub fn size(&self) -> u32 {
        4 >> self.t as u32
    }

    pub fn set_z(&mut self, value: u32) {
        self.z = value == 0
    }

    pub fn set_n(&mut self, value: u32) {
        self.n = (value & 0x8000000) == 1
    }

    pub fn set_c_add(&mut self, op1: u64, op2: u64) {
        self.c = op1 + op2 > 0xFFFF_FFFF;
    }

    pub fn set_c_sub(&mut self, op1: u64, op2: u64) {
        self.c = op2 <= op1;
    }

    pub fn set_v_add(&mut self, op1: u32, op2: u32, res: u32) {
        self.v = ((op1 ^ res) & (!op1 ^ op2)) & 0x8000000 == 1;
    }

    pub fn set_v_sub(&mut self, op1: u32, op2: u32, res: u32) {
        self.v = ((op1 ^ op2) & (!op2 ^ res)) & 0x8000000 == 1;
    }
}

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