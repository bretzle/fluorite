use modular_bitfield::prelude::*;
use std::fmt;

static_assertions::assert_eq_size!(StatusRegister, u32);

#[bitfield]
#[repr(u32)]
#[derive(Clone, Copy)]
pub struct StatusRegister {
    pub mode: CpuMode,
    pub state: CpuState,
    pub fiq_disable: bool,
    pub irq_disable: bool,

    #[skip]
    _reserved: B20,

    pub v: bool,
    pub c: bool,
    pub z: bool,
    pub n: bool,
}

#[derive(BitfieldSpecifier, Copy, Clone, Debug, PartialEq)]
pub enum CpuState {
    ARM = 0,
    THUMB = 1,
}

impl fmt::Display for CpuState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use CpuState::*;
        match self {
            ARM => write!(f, "ARM"),
            THUMB => write!(f, "THUMB"),
        }
    }
}

#[derive(BitfieldSpecifier, Copy, Clone, Debug, PartialEq)]
#[repr(u32)]
#[bits = 5]
pub enum CpuMode {
    User = 0b10000,
    Fiq = 0b10001,
    Irq = 0b10010,
    Supervisor = 0b10011,
    Abort = 0b10111,
    Undefined = 0b11011,
    System = 0b11111,
}

impl fmt::Display for CpuMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use CpuMode::*;
        match self {
            User => write!(f, "USR"),
            Fiq => write!(f, "FIQ"),
            Irq => write!(f, "IRQ"),
            Supervisor => write!(f, "SVC"),
            Abort => write!(f, "ABT"),
            Undefined => write!(f, "UND"),
            System => write!(f, "SYS"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    impl StatusRegister {
        fn raw(&self) -> u32 {
            u32::from_le_bytes(self.clone().into_bytes())
        }
    }

    #[test]
    fn cpu_mode() {
        let mut reg = StatusRegister::new();

        assert_eq!(reg.raw(), 0);

        reg.set_mode(CpuMode::User);

        assert_eq!(reg.raw(), 0x00000010);

        reg.set_c(true);

        assert_eq!(reg.raw(), 0x20000010);
    }
}
