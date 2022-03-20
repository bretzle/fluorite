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
    BIOS,
    EWRAM,
    IWRAM,
    IO,
    Palette,
    VRAM,
    OAM,
    ROM0L,
    ROM0H,
    ROM1L,
    ROM1H,
    ROM2L,
    ROM2H,
    SRAM,
    Unused,
}

impl MemoryRegion {
    pub fn get_region(addr: u32) -> MemoryRegion {
        match addr >> 24 {
            0x00 if addr < 0x4000 => MemoryRegion::BIOS, // Not Mirrored
            0x02 => MemoryRegion::EWRAM,
            0x03 => MemoryRegion::IWRAM,
            0x04 => MemoryRegion::IO,
            0x05 => MemoryRegion::Palette,
            0x06 => MemoryRegion::VRAM,
            0x07 => MemoryRegion::OAM,
            0x08 => MemoryRegion::ROM0L,
            0x09 => MemoryRegion::ROM0H,
            0x0A => MemoryRegion::ROM1L,
            0x0B => MemoryRegion::ROM1H,
            0x0C => MemoryRegion::ROM2L,
            0x0D => MemoryRegion::ROM2H,
            0x0E | 0x0F => MemoryRegion::SRAM,
            _ => MemoryRegion::Unused,
        }
    }
}
