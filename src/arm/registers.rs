use bitflags::bitflags;

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
    CPSR,
    SPSR,
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

    pub fn get_reg(&self, reg: Reg) -> u32 {
        let mode = self.cpsr.get_mode();
        use Reg::*;
        match reg {
            R0 | R1 | R2 | R3 | R4 | R5 | R6 | R7 => self.usr[reg as usize],
            R8 | R9 | R10 | R11 | R12 => match mode {
                Mode::FIQ => self.fiq[reg as usize - 8],
                _ => self.usr[reg as usize],
            },
            R13 | R14 => match mode {
                Mode::FIQ => self.fiq[reg as usize - 8],
                Mode::SVC => self.svc[reg as usize - 13],
                Mode::ABT => self.abt[reg as usize - 13],
                Mode::IRQ => self.irq[reg as usize - 13],
                Mode::UND => self.und[reg as usize - 13],
                _ => self.usr[reg as usize],
            },
            R15 => self.pc,
            CPSR => self.cpsr.bits,
            SPSR => match mode {
                Mode::FIQ => self.spsr[0].bits(),
                Mode::SVC => self.spsr[1].bits(),
                Mode::ABT => self.spsr[2].bits(),
                Mode::IRQ => self.spsr[3].bits(),
                Mode::UND => self.spsr[4].bits(),
                _ => self.cpsr.bits(),
            },
        }
    }

    pub fn set_reg(&mut self, reg: Reg, value: u32) {
        let mode = self.cpsr.get_mode();
        use Reg::*;
        match reg {
            R0 | R1 | R2 | R3 | R4 | R5 | R6 | R7 => self.usr[reg as usize] = value,
            R8 | R9 | R10 | R11 | R12 => match mode {
                Mode::FIQ => self.fiq[reg as usize - 8] = value,
                _ => self.usr[reg as usize] = value,
            },
            R13 | R14 => match mode {
                Mode::FIQ => self.fiq[reg as usize - 8] = value,
                Mode::SVC => self.svc[reg as usize - 13] = value,
                Mode::ABT => self.abt[reg as usize - 13] = value,
                Mode::IRQ => self.irq[reg as usize - 13] = value,
                Mode::UND => self.und[reg as usize - 13] = value,
                _ => self.usr[reg as usize] = value,
            },
            R15 => self.pc = value,
            CPSR => self.cpsr.bits = value,
            SPSR => match mode {
                Mode::FIQ => self.spsr[0] = StatusRegister::from_bits(value).unwrap(),
                Mode::SVC => self.spsr[1] = StatusRegister::from_bits(value).unwrap(),
                Mode::ABT => self.spsr[2] = StatusRegister::from_bits(value).unwrap(),
                Mode::IRQ => self.spsr[3] = StatusRegister::from_bits(value).unwrap(),
                Mode::UND => self.spsr[4] = StatusRegister::from_bits(value).unwrap(),
                _ => (),
            },
        }
    }

    pub fn _get_n(&self) -> bool {
        self.cpsr.contains(StatusRegister::N)
    }
    pub fn _get_z(&self) -> bool {
        self.cpsr.contains(StatusRegister::Z)
    }
    pub fn get_c(&self) -> bool {
        self.cpsr.contains(StatusRegister::C)
    }
    pub fn _get_v(&self) -> bool {
        self.cpsr.contains(StatusRegister::V)
    }
    pub fn get_i(&self) -> bool {
        self.cpsr.contains(StatusRegister::I)
    }
    pub fn _get_f(&self) -> bool {
        self.cpsr.contains(StatusRegister::F)
    }
    pub fn get_flags(&self) -> u32 {
        self.cpsr.bits >> 24
    }
    pub fn get_t(&self) -> bool {
        self.cpsr.contains(StatusRegister::T)
    }
    pub fn get_mode(&self) -> Mode {
        self.cpsr.get_mode()
    }
    pub fn set_n(&mut self, value: bool) {
        self.cpsr.set(StatusRegister::N, value)
    }
    pub fn set_z(&mut self, value: bool) {
        self.cpsr.set(StatusRegister::Z, value)
    }
    pub fn set_c(&mut self, value: bool) {
        self.cpsr.set(StatusRegister::C, value)
    }
    pub fn set_v(&mut self, value: bool) {
        self.cpsr.set(StatusRegister::V, value)
    }
    pub fn set_i(&mut self, value: bool) {
        self.cpsr.set(StatusRegister::I, value)
    }
    pub fn _set_f(&mut self, value: bool) {
        self.cpsr.set(StatusRegister::F, value)
    }
    pub fn set_t(&mut self, value: bool) {
        self.cpsr.set(StatusRegister::T, value)
    }
    pub fn set_mode(&mut self, mode: Mode) {
        self.cpsr.set_mode(mode)
    }
}

#[derive(PartialEq)]
pub enum Mode {
    USR = 0b10000,
    FIQ = 0b10001,
    IRQ = 0b10010,
    SVC = 0b10011,
    ABT = 0b10111,
    SYS = 0b11111,
    UND = 0b11011,
}

bitflags! {
    struct StatusRegister: u32 {
        const N =  0x80000000;
        const Z =  0x40000000;
        const C =  0x20000000;
        const V =  0x10000000;
        const F =  0x00000040;
        const I =  0x00000080;
        const T =  0x00000020;
        const M4 = 0x00000010;
        const M3 = 0x00000008;
        const M2 = 0x00000004;
        const M1 = 0x00000002;
        const M0 = 0x00000001;
    }
}

impl StatusRegister {
    pub fn reset() -> Self {
        Self::from_bits(Mode::SYS as u32).unwrap()
    }

    pub fn get_mode(&self) -> Mode {
        match Some(self.bits() & 0x1F) {
            Some(m) if m == Mode::USR as u32 => Mode::USR,
            Some(m) if m == Mode::FIQ as u32 => Mode::FIQ,
            Some(m) if m == Mode::IRQ as u32 => Mode::IRQ,
            Some(m) if m == Mode::SVC as u32 => Mode::SVC,
            Some(m) if m == Mode::ABT as u32 => Mode::ABT,
            Some(m) if m == Mode::SYS as u32 => Mode::SYS,
            Some(m) if m == Mode::UND as u32 => Mode::UND,
            _ => panic!("Invalid Mode"),
        }
    }

    pub fn set_mode(&mut self, mode: Mode) {
        self.bits = (self.bits() & !0x1F) | mode as u32;
    }
}
