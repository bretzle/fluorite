use crate::sysbus::Bus;
use fluorite_arm::Addr;
use std::fmt;

const VRAM_OBJ_TILES_START_TEXT: u32 = 0x1_0000;
const VRAM_OBJ_TILES_START_BITMAP: u32 = 0x1_4000;

pub struct Gpu {
    pub dispcnt: DisplayControl,

	pub palette_ram: Vec<u8>,
	pub vram: Vec<u8>,
	pub oam: Vec<u8>,

	pub(crate) vram_obj_tiles_start: u32,
}

impl Gpu {
    pub fn new() -> Self {
        Self {
            dispcnt: DisplayControl::from(0x80),

			palette_ram: vec![0; 1 * 1024],
			vram: vec![0; 128 * 1024],
			oam: vec![0;  1 * 1024],

			vram_obj_tiles_start: VRAM_OBJ_TILES_START_TEXT,
        }
    }

    pub fn write_dispcnt(&mut self, val: u16) {
        let old = self.dispcnt.mode();
        self.dispcnt = val.into();
        let new = self.dispcnt.mode();

        if old != new {
            println!("[GPU] Display mode changed! {} -> {}", old, new);
            self.vram_obj_tiles_start = if new as u8 >= 3 {
                VRAM_OBJ_TILES_START_BITMAP
            } else {
                VRAM_OBJ_TILES_START_TEXT
            };
        }
    }
}

impl Bus for Gpu {
    fn read_8(&mut self, addr: Addr) -> u8 {
        todo!()
    }

    fn write_8(&mut self, addr: Addr, val: u8) {
        todo!()
    }

    fn write_16(&mut self, addr: Addr, val: u16) {
        todo!()
    }
}

use modular_bitfield::prelude::*;

static_assertions::assert_eq_size!(DisplayControl, u16);

#[bitfield]
#[repr(u16)]
#[derive(Debug, Clone, Default)]
pub struct DisplayControl {
    pub mode: LcdMode,
    #[skip]
    _reserved: B1, // TODO: This is used for cgb compatibility
    pub display_frame_select: bool,
    pub hblank_interval_free: bool,
    pub obj_character_vram_mapping: bool,
    pub force_blank: bool,
    pub enable_bg0: bool,
    pub enable_bg1: bool,
    pub enable_bg2: bool,
    pub enable_bg3: bool,
    pub enable_obj: bool,
    pub enable_window0: bool,
    pub enable_window1: bool,
    pub enable_obj_window: bool,
}

#[derive(BitfieldSpecifier, Clone, Debug, PartialEq)]
#[bits = 3]
#[repr(u8)]
pub enum LcdMode {
    Mode0 = 0b000,
    Mode1 = 0b001,
    Mode2 = 0b010,
    Mode3 = 0b011,
    Mode4 = 0b100,
    Mode5 = 0b101,
    Prohibited,
}

impl fmt::Display for LcdMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LcdMode::Mode0 => write!(f, "0"),
            LcdMode::Mode1 => write!(f, "1"),
            LcdMode::Mode2 => write!(f, "2"),
            LcdMode::Mode3 => write!(f, "3"),
            LcdMode::Mode4 => write!(f, "4"),
            LcdMode::Mode5 => write!(f, "5"),
            LcdMode::Prohibited => write!(f, "prohibited"),
        }
    }
}
