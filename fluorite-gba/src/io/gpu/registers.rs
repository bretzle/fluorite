use crate::io::scheduler::Scheduler;
use bitflags::bitflags;
use std::ops::{Deref, DerefMut};

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

bitflags! {
    pub struct DISPSTATFlags: u16 {
        const VBLANK = 1;
        const HBLANK = 1 << 1;
        const VCOUNTER = 1 << 2;
        const VBLANK_IRQ_ENABLE = 1 << 3;
        const HBLANK_IRQ_ENABLE = 1 << 4;
        const VCOUNTER_IRQ_ENALBE = 1 << 5;
    }
}

pub struct Dispstat {
    pub flags: DISPSTATFlags,
    pub vcount_setting: u8,
}

impl Dispstat {
    pub fn new() -> Self {
        Self {
            flags: DISPSTATFlags::empty(),
            vcount_setting: 0,
        }
    }
}

impl Deref for Dispstat {
    type Target = DISPSTATFlags;

    fn deref(&self) -> &DISPSTATFlags {
        &self.flags
    }
}

impl DerefMut for Dispstat {
    fn deref_mut(&mut self) -> &mut DISPSTATFlags {
        &mut self.flags
    }
}

impl Dispstat {
    pub fn read(&self, byte: u8) -> u8 {
        match byte {
            0 => self.flags.bits as u8,
            1 => self.vcount_setting as u8,
            _ => unreachable!(),
        }
    }

    pub fn write(&mut self, _scheduler: &mut Scheduler, byte: u8, value: u8) {
        match byte {
            0 => {
                let old_bits = self.flags.bits;
                self.flags.bits =
                    self.flags.bits & 0x7 | ((value as u16) & !0x7 & DISPSTATFlags::all().bits);
                assert_eq!(old_bits & 0x7, self.flags.bits & 0x7);
            }
            1 => self.vcount_setting = value as u8,
            _ => unreachable!(),
        }
    }
}

#[derive(Clone, Copy)]
pub struct BgCnt {
    pub priority: u8,
    pub tile_block: u8,
    pub mosaic: bool,
    pub bpp8: bool,
    pub map_block: u8,
    pub wrap: bool,
    pub screen_size: u8,
}

impl BgCnt {
    pub fn new() -> Self {
        Self {
            priority: 0,
            tile_block: 0,
            mosaic: false,
            bpp8: false,
            map_block: 0,
            wrap: false,
            screen_size: 0,
        }
    }

    pub fn read(&self, byte: u8) -> u8 {
        match byte {
            0 => {
                (self.bpp8 as u8) << 7
                    | (self.mosaic as u8) << 6
                    | self.tile_block << 2
                    | self.priority
            }
            1 => self.screen_size << 6 | (self.wrap as u8) << 5 | self.map_block,
            _ => unreachable!(),
        }
    }

    pub fn write(&mut self, byte: u8, value: u8) {
        match byte {
            0 => {
                self.priority = value & 0x3;
                self.tile_block = value >> 2 & 0x3;
                self.mosaic = value >> 6 & 0x1 != 0;
                self.bpp8 = value >> 7 & 0x1 != 0;
            }
            1 => {
                self.map_block = value & 0x1F;
                self.wrap = value >> 5 & 0x1 != 0;
                self.screen_size = value >> 6 & 0x3;
            }
            _ => unreachable!(),
        }
    }
}

pub struct MosaicSize {
    pub h_size: u8,
    pub v_size: u8,
}

impl MosaicSize {
    pub fn new() -> MosaicSize {
        MosaicSize {
            h_size: 1,
            v_size: 1,
        }
    }

    pub fn _read(&self) -> u8 {
        (self.v_size - 1) << 4 | (self.h_size - 1)
    }

    pub fn _write(&mut self, value: u8) {
        self.h_size = (value & 0xF) + 1;
        self.v_size = (value >> 4) + 1;
    }
}

pub struct Mosaic {
    pub bg_size: MosaicSize,
    pub obj_size: MosaicSize,
}

impl Mosaic {
    pub fn new() -> Self {
        Self {
            bg_size: MosaicSize::new(),
            obj_size: MosaicSize::new(),
        }
    }

    fn _read(&self, byte: u8) -> u8 {
        match byte {
            0 => self.bg_size._read(),
            1 => self.obj_size._read(),
            _ => unreachable!(),
        }
    }

    pub fn write(&mut self, _scheduler: &mut Scheduler, byte: u8, value: u8) {
        match byte {
            0 => self.bg_size._write(value),
            1 => self.obj_size._write(value),
            _ => unreachable!(),
        }
    }
}

pub struct BldCntTargetPixelSelection {
    pub enabled: [bool; 6],
}

impl BldCntTargetPixelSelection {
    pub fn new() -> BldCntTargetPixelSelection {
        BldCntTargetPixelSelection {
            enabled: [false; 6],
        }
    }

    pub fn _read(&self) -> u8 {
        (self.enabled[0] as u8)
            | (self.enabled[1] as u8) << 1
            | (self.enabled[2] as u8) << 2
            | (self.enabled[3] as u8) << 3
            | (self.enabled[4] as u8) << 4
            | (self.enabled[5] as u8) << 5
    }

    pub fn _write(&mut self, value: u8) {
        self.enabled[0] = value & 0x1 != 0;
        self.enabled[1] = value >> 1 & 0x1 != 0;
        self.enabled[2] = value >> 2 & 0x1 != 0;
        self.enabled[3] = value >> 3 & 0x1 != 0;
        self.enabled[4] = value >> 4 & 0x1 != 0;
        self.enabled[5] = value >> 5 & 0x1 != 0;
    }
}

#[derive(Clone, Copy)]
pub enum ColorSFX {
    None = 0,
    AlphaBlend = 1,
    _BrightnessInc = 2,
    _BrightnessDec = 3,
}

impl ColorSFX {
    pub fn _from(value: u8) -> ColorSFX {
        use ColorSFX::*;
        match value {
            0 => None,
            1 => AlphaBlend,
            2 => _BrightnessInc,
            3 => _BrightnessDec,
            _ => unreachable!(),
        }
    }
}

pub struct BldCnt {
    pub target_pixel1: BldCntTargetPixelSelection,
    pub effect: ColorSFX,
    pub target_pixel2: BldCntTargetPixelSelection,
}

impl BldCnt {
    pub fn new() -> Self {
        Self {
            target_pixel1: BldCntTargetPixelSelection::new(),
            effect: ColorSFX::None,
            target_pixel2: BldCntTargetPixelSelection::new(),
        }
    }

    fn _read(&self, byte: u8) -> u8 {
        match byte {
            0 => (self.effect as u8) << 6 | self.target_pixel1._read(),
            1 => self.target_pixel2._read(),
            _ => unreachable!(),
        }
    }

    pub fn write(&mut self, _scheduler: &mut Scheduler, byte: u8, value: u8) {
        match byte {
            0 => {
                self.target_pixel1._write(value);
                self.effect = ColorSFX::_from(value >> 6);
            }
            1 => self.target_pixel2._write(value),
            _ => unreachable!(),
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub struct WindowControl {
    pub bg0_enable: bool,
    pub bg1_enable: bool,
    pub bg2_enable: bool,
    pub bg3_enable: bool,
    pub obj_enable: bool,
    pub color_special_enable: bool,
}

impl WindowControl {
    pub fn new() -> Self {
        Self {
            bg0_enable: false,
            bg1_enable: false,
            bg2_enable: false,
            bg3_enable: false,
            obj_enable: false,
            color_special_enable: false,
        }
    }

    pub fn all() -> Self {
        Self {
            bg0_enable: true,
            bg1_enable: true,
            bg2_enable: true,
            bg3_enable: true,
            obj_enable: true,
            color_special_enable: true,
        }
    }

    fn _read(&self, byte: u8) -> u8 {
        match byte {
            0 => {
                (self.color_special_enable as u8) << 5
                    | (self.obj_enable as u8) << 4
                    | (self.bg3_enable as u8) << 3
                    | (self.bg2_enable as u8) << 2
                    | (self.bg1_enable as u8) << 1
                    | (self.bg0_enable as u8)
            }
            _ => unreachable!(),
        }
    }

    pub fn write(&mut self, _scheduler: &mut Scheduler, byte: u8, value: u8) {
        match byte {
            0 => {
                self.color_special_enable = value >> 5 & 0x1 != 0;
                self.obj_enable = value >> 4 & 0x1 != 0;
                self.bg3_enable = value >> 3 & 0x1 != 0;
                self.bg2_enable = value >> 2 & 0x1 != 0;
                self.bg1_enable = value >> 1 & 0x1 != 0;
                self.bg0_enable = value & 0x1 != 0;
            }
            _ => unreachable!(),
        }
    }
}

pub struct BldAlpha {
    _raw_eva: u8,
    _raw_evb: u8,
    pub eva: u16,
    pub evb: u16,
}

impl BldAlpha {
    pub fn new() -> Self {
        Self {
            _raw_eva: 0,
            _raw_evb: 0,
            eva: 0,
            evb: 0,
        }
    }

    fn _read(&self, byte: u8) -> u8 {
        match byte {
            0 => self._raw_eva,
            1 => self._raw_evb,
            _ => unreachable!(),
        }
    }

    pub fn write(&mut self, _scheduler: &mut Scheduler, byte: u8, value: u8) {
        match byte {
            0 => {
                self._raw_eva = value & 0x1F;
                self.eva = std::cmp::min(0x10, self._raw_eva as u16);
            }
            1 => {
                self._raw_evb = value & 0x1F;
                self.evb = std::cmp::min(0x10, self._raw_evb as u16);
            }
            _ => unreachable!(),
        }
    }
}

pub struct Bldy {
    pub evy: u8,
}

impl Bldy {
    pub fn new() -> Bldy {
        Bldy { evy: 0 }
    }

    fn _read(&self, _byte: u8) -> u8 {
        0
    }

    pub fn write(&mut self, __scheduler: &mut Scheduler, byte: u8, value: u8) {
        match byte {
            0 => self.evy = std::cmp::min(0x10, value & 0x1F),
            1 => (),
            _ => unreachable!(),
        }
    }
}

#[derive(Clone, Copy)]
pub struct Ofs(pub u16);

impl Ofs {
    pub fn new() -> Self {
        Self(0)
    }

    fn _read(&self, byte: u8) -> u8 {
        match byte {
            0 => self.0 as u8,
            1 => (self.0 >> 8) as u8,
            _ => unreachable!(),
        }
    }

    pub fn write(&mut self, _scheduler: &mut Scheduler, byte: u8, value: u8) {
        match byte {
            0 => self.0 = self.0 & !0xFF | value as u16,
            1 => self.0 = self.0 & !0x100 | (value as u16) << 8 & 0x100,
            _ => unreachable!(),
        }
    }
}

#[derive(Clone, Copy)]
pub struct ReferencePointCoord(i32);

impl ReferencePointCoord {
    pub fn new() -> Self {
        Self(0)
    }

    pub fn integer(&self) -> i32 {
        self.0 >> 8
    }

    fn _read(&self, _byte: u8) -> u8 {
        0
    }

    pub fn write(&mut self, _scheduler: &mut Scheduler, byte: u8, value: u8) {
        let offset = byte * 8;
        match byte {
            0..=2 => self.0 = (self.0 as u32 & !(0xFF << offset) | (value as u32) << offset) as i32,
            3 => {
                self.0 =
                    (self.0 as u32 & !(0xFF << offset) | (value as u32 & 0xF) << offset) as i32;
                if self.0 & 0x0800_0000 != 0 {
                    self.0 = ((self.0 as u32) | 0xF000_0000) as i32
                }
            }
            _ => unreachable!(),
        }
    }
}

impl std::ops::AddAssign<RotationScalingParameter> for ReferencePointCoord {
    fn add_assign(&mut self, rhs: RotationScalingParameter) {
        // *self = Self(self.value.wrapping_add(rhs.value as i32))
        self.0 = self.0.wrapping_add(rhs.0 as i32)
    }
}

#[derive(Clone, Copy)]
pub struct RotationScalingParameter(i16);

impl RotationScalingParameter {
    pub fn new() -> Self {
        Self(0)
    }

    pub fn get_float_from_u16(value: u16) -> f64 {
        (value >> 8) as i8 as i32 as f64 + value as u8 as f64 / 256.0
    }

    fn _read(&self, _byte: u8) -> u8 {
        0
    }

    pub fn write(&mut self, _scheduler: &mut Scheduler, byte: u8, value: u8) {
        let offset = byte * 8;
        match byte {
            0 | 1 => {
                self.0 = ((self.0 as u32) & !(0xFF << offset) | (value as u32) << offset) as i16
            }
            _ => unreachable!(),
        }
    }
}

#[derive(Clone, Copy)]
pub struct WindowDimensions {
    pub coord2: u8,
    pub coord1: u8,
}

impl WindowDimensions {
    pub fn new() -> Self {
        Self {
            coord2: 0,
            coord1: 0,
        }
    }

    fn read(&self, byte: u8) -> u8 {
        match byte {
            0 => self.coord2,
            1 => self.coord1,
            _ => unreachable!(),
        }
    }

    pub fn write(&mut self, _scheduler: &mut Scheduler, byte: u8, value: u8) {
        match byte {
            0 => self.coord2 = value,
            1 => self.coord1 = value,
            _ => unreachable!(),
        }
    }
}
