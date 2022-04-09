use fluorite_common::bitfield;
use std::ops::BitOrAssign;

bitfield! {
    pub struct InterruptEnable(u16) {
        pub raw: u16 [read_only] @ ..,
        pub vblank: bool @ 0,
        pub hblank: bool @ 1,
        pub vcounter_match: bool @ 2,
        pub timer0_overflow: bool @ 3,
        pub timer1_overflow: bool @ 4,
        pub timer2_overflow: bool @ 5,
        pub timer3_overflow: bool @ 6,
        serial: bool @ 7,
        pub dma0: bool @ 8,
        pub dma1: bool @ 9,
        pub dma2: bool @ 10,
        pub dma3: bool @ 11,
        keypad: bool @ 12,
        gamepak: bool @ 13,
    }
}

impl InterruptEnable {
    pub const fn new() -> Self {
        Self(0)
    }

    pub fn read<const BYTE: u8>(&self) -> u8 {
        match BYTE {
            0 => self.0 as u8,
            1 => (self.0 >> 8) as u8,
            _ => unreachable!(),
        }
    }

    pub fn write<const BYTE: u8>(&mut self, value: u8) {
        self.0 = match BYTE {
            0 => self.0 & !0x00FF | (value as u16) & 0x3FFF,
            1 => self.0 & !0xFF00 | (value as u16) << 8 & 0x3FFF,
            _ => unreachable!(),
        }
    }
}

bitfield! {
    pub struct InterruptMasterEnable(u16) {
        pub enabled: bool @ 0,
    }
}

impl InterruptMasterEnable {
    pub const fn new() -> Self {
        Self(0)
    }

    pub fn read<const BYTE: u8>(&self) -> u8 {
        match BYTE {
            0 => self.0 as u8,
            1 => (self.0 >> 8) as u8,
            _ => unreachable!(),
        }
    }

    pub fn write<const BYTE: u8>(&mut self, val: u8) {
        self.0 = match BYTE {
            0 => self.0 & !0x00FF | (val as u16) & 1,
            1 => self.0 & !0xFF00 | (val as u16) << 8 & 1,
            _ => unreachable!(),
        }
    }
}

bitfield! {
    #[derive(Clone, Copy)]
    pub struct InterruptRequest(u16) {
        pub raw: u16 [read_only] @ ..,
        pub vblank: bool @ 0,
        pub hblank: bool @ 1,
        pub vcounter_match: bool @ 2,
        pub timer0_overflow: bool @ 3,
        pub timer1_overflow: bool @ 4,
        pub timer2_overflow: bool @ 5,
        pub timer3_overflow: bool @ 6,
        serial: bool @ 7,
        pub dma0: bool @ 8,
        pub dma1: bool @ 9,
        pub dma2: bool @ 10,
        pub dma3: bool @ 11,
        keypad: bool @ 12,
        gamepak: bool @ 13,
    }
}

impl InterruptRequest {
    pub const fn new() -> Self {
        Self(0)
    }

    pub fn read<const BYTE: u8>(&self) -> u8 {
        match BYTE {
            0 => self.0 as u8,
            1 => (self.0 >> 8) as u8,
            _ => unreachable!(),
        }
    }

    pub fn write<const BYTE: u8>(&mut self, value: u8) {
        self.0 &= match BYTE {
            0 => !(value as u16),
            1 => !((value as u16) << 8),
            _ => unreachable!(),
        }
    }
}

impl BitOrAssign for InterruptRequest {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0
    }
}
