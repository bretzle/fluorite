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
        self.cpsr.bits = 0x5F;
    }

    pub fn get_reg(&self, reg: Reg) -> u32 {
        let mode = self.cpsr.get_mode();
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
            Cpsr => self.cpsr.bits,
            Spsr => match mode {
                Mode::Fiq => self.spsr[0].bits(),
                Mode::Supervisor => self.spsr[1].bits(),
                Mode::Abort => self.spsr[2].bits(),
                Mode::Irq => self.spsr[3].bits(),
                Mode::Undefined => self.spsr[4].bits(),
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
            Cpsr => self.cpsr.bits = value,
            Spsr => match mode {
                Mode::Fiq => self.spsr[0] = StatusRegister::from_bits(value).unwrap(),
                Mode::Supervisor => self.spsr[1] = StatusRegister::from_bits(value).unwrap(),
                Mode::Abort => self.spsr[2] = StatusRegister::from_bits(value).unwrap(),
                Mode::Irq => self.spsr[3] = StatusRegister::from_bits(value).unwrap(),
                Mode::Undefined => self.spsr[4] = StatusRegister::from_bits(value).unwrap(),
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
        self.cpsr.bits = self.get_reg(Reg::Spsr);
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

    pub fn get_n(&self) -> bool {
        self.cpsr.contains(StatusRegister::N)
    }
    pub fn get_z(&self) -> bool {
        self.cpsr.contains(StatusRegister::Z)
    }
    pub fn get_c(&self) -> bool {
        self.cpsr.contains(StatusRegister::C)
    }
    pub fn get_v(&self) -> bool {
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
        Self::from_bits(Mode::System as u32).unwrap()
    }

    pub fn get_mode(&self) -> Mode {
        match self.bits() & 0x1F {
            m if m == Mode::User as u32 => Mode::User,
            m if m == Mode::Fiq as u32 => Mode::Fiq,
            m if m == Mode::Irq as u32 => Mode::Irq,
            m if m == Mode::Supervisor as u32 => Mode::Supervisor,
            m if m == Mode::Abort as u32 => Mode::Abort,
            m if m == Mode::System as u32 => Mode::System,
            m if m == Mode::Undefined as u32 => Mode::Undefined,
            bits => panic!("Invalid Mode: {bits:05b}"),
        }
    }

    pub fn set_mode(&mut self, mode: Mode) {
        self.bits = (self.bits() & !0x1F) | mode as u32;
    }
}
