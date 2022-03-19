use bitflags::bitflags;
use std::ops::{Deref, DerefMut};

use crate::io::scheduler::Scheduler;

#[derive(Clone, Copy, PartialEq)]
pub enum BGMode {
    Mode0 = 0,
    Mode1 = 1,
    Mode2 = 2,
    Mode3 = 3,
    Mode4 = 4,
    Mode5 = 5,
}

impl BGMode {
    pub fn get(mode: u8) -> BGMode {
        use BGMode::*;
        match mode {
            0 => Mode0,
            1 => Mode1,
            2 => Mode2,
            3 => Mode3,
            4 => Mode4,
            5 => Mode5,
            _ => panic!("Invalid BG Mode!"),
        }
    }
}

bitflags! {
    pub struct DISPCNTFlags: u16 {
        const CGB_MODE = 1 << 3;
        const DISPLAY_FRAME_SELECT = 1 << 4;
        const HBLANK_INTERVAL_FREE = 1 << 5;
        const OBJ_TILES1D = 1 << 6;
        const FORCED_BLANK = 1 << 7;
        const DISPLAY_BG0 = 1 << 8;
        const DISPLAY_BG1 = 1 << 9;
        const DISPLAY_BG2 = 1 << 10;
        const DISPLAY_BG3 = 1 << 11;
        const DISPLAY_OBJ = 1 << 12;
        const DISPLAY_WINDOW0 = 1 << 13;
        const DISPLAY_WINDOW1 = 1 << 14;
        const DISPLAY_OBJ_WINDOW = 1 << 15;
    }
}

pub struct Dispcnt {
    pub flags: DISPCNTFlags,
    pub mode: BGMode,
}

impl Dispcnt {
    pub fn new() -> Self {
        Self {
            flags: DISPCNTFlags::empty(),
            mode: BGMode::Mode0,
        }
    }

    pub fn windows_enabled(&self) -> bool {
        (self.bits() >> 13) != 0
    }

    pub fn read(&self, byte: u8) -> u8 {
        match byte {
            0 => (self.flags.bits as u8) | (self.mode as u8),
            1 => (self.flags.bits >> 8) as u8,
            _ => unreachable!(),
        }
    }

    pub fn write(&mut self, byte: u8, value: u8) {
        match byte {
            0 => {
                self.mode = BGMode::get(value & 0x7);
                self.flags.bits =
                    self.flags.bits & !0x00FF | (value as u16) & DISPCNTFlags::all().bits;
            }
            1 => {
                self.flags.bits =
                    self.flags.bits & !0xFF00 | (value as u16) << 8 & DISPCNTFlags::all().bits
            }
            _ => unreachable!(),
        }
    }
}

impl Deref for Dispcnt {
    type Target = DISPCNTFlags;

    fn deref(&self) -> &DISPCNTFlags {
        &self.flags
    }
}

impl DerefMut for Dispcnt {
    fn deref_mut(&mut self) -> &mut DISPCNTFlags {
        &mut self.flags
    }
}
