use super::Sysbus;
use num::{FromPrimitive, PrimInt};

pub trait MemoryValue: PrimInt + FromPrimitive {}

impl MemoryValue for u8 {}
impl MemoryValue for u16 {}
impl MemoryValue for u32 {}

impl Sysbus {
    pub fn setup_openbus(&mut self, pc: u32, in_thumb: bool, pipeline: &[u32; 2]) {
        self.pc = pc;
        self.in_thumb = in_thumb;
        self.pipeline = *pipeline;
    }
}

#[derive(PartialEq)]
pub enum MemoryRegion {
    Bios,
    Ewram,
    Iwram,
    Io,
    Palette,
    Vram,
    Oam,
    Rom0L,
    Rom0H,
    Rom1L,
    Rom1H,
    Rom2L,
    Rom2H,
    Sram,
    Unused,
}

impl MemoryRegion {
    pub fn get_region(addr: u32) -> MemoryRegion {
        match addr >> 24 {
            0x00 if addr < 0x4000 => MemoryRegion::Bios, // Not Mirrored
            0x02 => MemoryRegion::Ewram,
            0x03 => MemoryRegion::Iwram,
            0x04 => MemoryRegion::Io,
            0x05 => MemoryRegion::Palette,
            0x06 => MemoryRegion::Vram,
            0x07 => MemoryRegion::Oam,
            0x08 => MemoryRegion::Rom0L,
            0x09 => MemoryRegion::Rom0H,
            0x0A => MemoryRegion::Rom1L,
            0x0B => MemoryRegion::Rom1H,
            0x0C => MemoryRegion::Rom2L,
            0x0D => MemoryRegion::Rom2H,
            0x0E | 0x0F => MemoryRegion::Sram,
            _ => MemoryRegion::Unused,
        }
    }
}
