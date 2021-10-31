use std::str::from_utf8;

use color_eyre::{eyre::eyre, Result};
use fluorite_arm::Addr;

use crate::sysbus::{Bus, SRAM_HI, SRAM_LO};

#[derive(Clone, Debug)]
pub struct Cartridge {
    pub header: CartridgeHeader,
    bytes: Box<[u8]>,
    size: usize,
}

impl Cartridge {
    pub fn new(bytes: &[u8]) -> Result<Self> {
        let header = CartridgeHeader::parse(bytes)?;
        let size = bytes.len();

        println!("{:#?}", header);

        Ok(Self {
            header,
            bytes: bytes.to_vec().into_boxed_slice(),
            size,
        })
    }

    fn read_unused(&self, addr: Addr) -> u8 {
        let x = (addr / 2) & 0xffff;
        if addr & 1 != 0 {
            (x >> 8) as u8
        } else {
            x as u8
        }
    }
}

#[derive(Clone, Debug)]
pub struct CartridgeHeader {
    pub game_title: String,
    pub game_code: String,
    pub maker_code: String,
    pub software_version: u8,
    pub checksum: u8,
}

impl CartridgeHeader {
    fn parse(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 0xC0 {
            return Err(eyre!("incomplete cartridge header"));
        }

        let checksum = bytes[0xbd];
        let calc = bytes[0xa0..=0xbc]
            .iter()
            .cloned()
            .fold(0, u8::wrapping_sub)
            .wrapping_sub(0x19);

        if calc != checksum {
            println!(
                "invalid header checksum, calculated {:02x} but expected {:02x}",
                calc, checksum
            );
        }

        let game_title = from_utf8(&bytes[0xa0..0xac]).map_err(|_| eyre!("invalid game title"))?;
        let game_code = from_utf8(&bytes[0xac..0xb0]).map_err(|_| eyre!("invalid game code"))?;
        let maker_code = from_utf8(&bytes[0xb0..0xb2]).map_err(|_| eyre!("invalid marker code"))?;

        Ok(Self {
            game_title: game_title.to_string(),
            game_code: game_code.to_string(),
            maker_code: maker_code.to_string(),
            software_version: bytes[0xBC],
            checksum,
        })
    }
}

impl Bus for Cartridge {
    fn read_8(&mut self, addr: Addr) -> u8 {
        let offset = (addr & 0x01FF_FFFF) as usize;
        match addr & 0xFF000000 {
            SRAM_LO | SRAM_HI => todo!(),
            _ => {
                if offset >= self.size {
                    self.read_unused(addr)
                } else {
                    self.bytes[offset as usize]
                }
            }
        }
    }

    fn write_8(&mut self, addr: Addr, _val: u8) {
        match addr & 0xFF000000 {
            SRAM_LO | SRAM_HI => {
                todo!()
            }
            _ => {}
        }
    }
}
