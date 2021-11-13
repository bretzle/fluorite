use fluorite_arm::Addr;

pub const WORK_RAM_SIZE: usize = 256 * 1024;
pub const INTERNAL_RAM_SIZE: usize = 32 * 1024;

pub const BIOS_ADDR: u32 = 0x0000_0000;
pub const EWRAM_ADDR: u32 = 0x0200_0000;
pub const IWRAM_ADDR: u32 = 0x0300_0000;
pub const IOMEM_ADDR: u32 = 0x0400_0000;
pub const PALRAM_ADDR: u32 = 0x0500_0000;
pub const VRAM_ADDR: u32 = 0x0600_0000;
pub const OAM_ADDR: u32 = 0x0700_0000;
pub const GAMEPAK_WS0_LO: u32 = 0x0800_0000;
pub const GAMEPAK_WS0_HI: u32 = 0x0900_0000;
pub const GAMEPAK_WS1_LO: u32 = 0x0A00_0000;
pub const GAMEPAK_WS1_HI: u32 = 0x0B00_0000;
pub const GAMEPAK_WS2_LO: u32 = 0x0C00_0000;
pub const GAMEPAK_WS2_HI: u32 = 0x0D00_0000;
pub const SRAM_LO: u32 = 0x0E00_0000;
pub const SRAM_HI: u32 = 0x0F00_0000;

pub const PAGE_BIOS: usize = (BIOS_ADDR >> 24) as usize;
pub const PAGE_EWRAM: usize = (EWRAM_ADDR >> 24) as usize;
pub const PAGE_IWRAM: usize = (IWRAM_ADDR >> 24) as usize;
pub const PAGE_IOMEM: usize = (IOMEM_ADDR >> 24) as usize;
pub const PAGE_PALRAM: usize = (PALRAM_ADDR >> 24) as usize;
pub const PAGE_VRAM: usize = (VRAM_ADDR >> 24) as usize;
pub const PAGE_OAM: usize = (OAM_ADDR >> 24) as usize;
pub const PAGE_GAMEPAK_WS0: usize = (GAMEPAK_WS0_LO >> 24) as usize;
pub const PAGE_GAMEPAK_WS1: usize = (GAMEPAK_WS1_LO >> 24) as usize;
pub const PAGE_GAMEPAK_WS2: usize = (GAMEPAK_WS2_LO >> 24) as usize;
pub const PAGE_SRAM_LO: usize = (SRAM_LO >> 24) as usize;
pub const PAGE_SRAM_HI: usize = (SRAM_HI >> 24) as usize;

pub const VIDEO_RAM_SIZE: usize = 128 * 1024;
pub const PALETTE_RAM_SIZE: usize = 1 * 1024;
pub const OAM_SIZE: usize = 1 * 1024;

pub const DISPLAY_WIDTH: usize = 240;
pub const DISPLAY_HEIGHT: usize = 160;
pub const VBLANK_LINES: usize = 68;

pub const CYCLES_PIXEL: usize = 4;
pub const CYCLES_HDRAW: usize = 960 + 46;
pub const CYCLES_HBLANK: usize = 272 - 46;
pub const CYCLES_SCANLINE: usize = 1232;
pub const CYCLES_VDRAW: usize = 197120;
pub const CYCLES_VBLANK: usize = 83776;

pub const CYCLES_FULL_REFRESH: usize = 280896;

pub const TILE_SIZE: u32 = 0x20;

pub const VRAM_OBJ_TILES_START_TEXT: u32 = 0x1_0000;
pub const VRAM_OBJ_TILES_START_BITMAP: u32 = 0x1_4000;

pub const IO_BASE: Addr = 0x0400_0000;
pub const REG_DISPCNT: Addr = 0x0400_0000;  // R/W  LCD Control
pub const REG_DISPSTAT: Addr = 0x0400_0004; // R/W  General LCD Status (STAT,LYC)
