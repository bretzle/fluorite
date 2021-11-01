use crate::{bios::Bios, cartridge::Cartridge, iodev::IoDevices};
use fluorite_arm::{memory::MemoryInterface, Addr};
use fluorite_common::Shared;

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

#[derive(Clone)]
pub struct SysBus {
    bios: Bios,
    ewram: Box<[u8]>,
    iwram: Box<[u8]>,
    cartridge: Cartridge,

	io: Shared<IoDevices>,
}

impl SysBus {
    pub fn new(bios: &[u8], rom: &[u8], io: &Shared<IoDevices>) -> Self {
        Self {
            bios: Bios::new(bios),
            ewram: vec![0; 256 * 1024].into_boxed_slice(),
            iwram: vec![0; 32 * 1024].into_boxed_slice(),
            cartridge: Cartridge::new(rom).unwrap(),
            io: io.clone(),
        }
    }

    fn read_invalid(&mut self, addr: Addr) -> u32 {
        panic!("invalid read @{:08X}", addr)
    }
}

pub trait Bus {
    fn read_8(&mut self, addr: Addr) -> u8;
    fn read_16(&mut self, addr: Addr) -> u16 {
        self.read_8(addr) as u16 | (self.read_8(addr + 1) as u16) << 8
    }
    fn read_32(&mut self, addr: Addr) -> u32 {
        self.read_16(addr) as u32 | (self.read_16(addr + 2) as u32) << 16
    }

    fn write_8(&mut self, addr: Addr, val: u8);
    fn write_16(&mut self, addr: Addr, val: u16) {
        self.write_8(addr, (val & 0xFF) as u8);
        self.write_8(addr + 1, ((val >> 8) & 0xFF) as u8);
    }
    fn write_32(&mut self, addr: Addr, val: u32) {
        self.write_16(addr, (val & 0xffff) as u16);
        self.write_16(addr + 2, (val >> 16) as u16);
    }
}

impl Bus for SysBus {
    fn read_8(&mut self, addr: Addr) -> u8 {
        todo!()
    }

    fn read_16(&mut self, addr: Addr) -> u16 {
        todo!()
    }

    fn read_32(&mut self, addr: Addr) -> u32 {
        match addr & 0xFF000000 {
            BIOS_ADDR => {
                if addr <= 0x3FFC {
                    self.bios.read_32(addr)
                } else {
                    self.read_invalid(addr)
                }
            }
            EWRAM_ADDR => self.ewram.read_32(addr & 0x3_FFFC),
            IWRAM_ADDR => self.iwram.read_32(addr & 0x7FFC),
            IOMEM_ADDR => {
                todo!()
            }
            PALRAM_ADDR | VRAM_ADDR | OAM_ADDR => todo!(),
            GAMEPAK_WS0_LO | GAMEPAK_WS0_HI | GAMEPAK_WS1_LO | GAMEPAK_WS1_HI | GAMEPAK_WS2_LO => {
                self.cartridge.read_32(addr)
            }
            GAMEPAK_WS2_HI => self.cartridge.read_32(addr),
            SRAM_LO | SRAM_HI => self.cartridge.read_32(addr),
            _ => self.read_invalid(addr),
        }
    }

    fn write_8(&mut self, addr: Addr, val: u8) {
        todo!()
    }

    fn write_16(&mut self, addr: Addr, val: u16) {
        match addr & 0xFF000000 {
            BIOS_ADDR => {}
            EWRAM_ADDR => self.ewram.write_16(addr & 0x3_fffe, val),
            IWRAM_ADDR => self.iwram.write_16(addr & 0x7ffe, val),
            IOMEM_ADDR => {
                let addr = if addr & 0xFFFE == 0x8000 {
                    0x800
                } else {
                    addr & 0x00FFFFFE
                };
                self.io.write_16(addr, val)
            }
            PALRAM_ADDR | VRAM_ADDR | OAM_ADDR => todo!(),
            GAMEPAK_WS0_LO => self.cartridge.write_16(addr, val),
            GAMEPAK_WS2_HI => self.cartridge.write_16(addr, val),
            SRAM_LO | SRAM_HI => self.cartridge.write_16(addr, val),
            _ => {
                println!("trying to write invalid address {:#x}", addr);
            }
        }
    }

    fn write_32(&mut self, addr: Addr, val: u32) {
        todo!()
    }
}

impl MemoryInterface for SysBus {
    fn load_8(&mut self, addr: Addr) -> u8 {
        self.read_8(addr)
    }

    fn load_16(&mut self, addr: Addr) -> u16 {
        self.read_16(addr)
    }

    fn load_32(&mut self, addr: Addr) -> u32 {
        self.read_32(addr)
    }

    fn store_8(&mut self, addr: Addr, val: u8) {
        self.write_8(addr, val);
    }

    fn store_16(&mut self, addr: Addr, val: u16) {
        self.write_16(addr, val);
    }

    fn store_32(&mut self, addr: Addr, val: u32) {
        self.write_32(addr, val);
    }
}

impl Bus for Box<[u8]> {
    fn read_8(&mut self, addr: Addr) -> u8 {
        self[addr as usize]
    }

    fn write_8(&mut self, addr: Addr, val: u8) {
        self[addr as usize] = val;
    }
}
