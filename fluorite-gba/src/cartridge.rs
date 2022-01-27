use crate::{consts::*, sysbus::Bus};
use fluorite_arm::Addr;
use std::str::from_utf8;

type Gpio = ();

#[derive(Clone, Debug)]
pub struct Cartridge {
    pub header: CartridgeHeader,
    bytes: Box<[u8]>,
    size: usize,
    backup: BackupMedia,
    gpio: Option<Gpio>,
}

impl Cartridge {
    pub fn new(bytes: &[u8]) -> Result<Self, String> {
        let header = CartridgeHeader::parse(bytes)?;
        let size = bytes.len();

        println!("{:#?}", header);

        Ok(Self {
            header,
            bytes: bytes.to_vec().into_boxed_slice(),
            size,
            backup: BackupMedia::Sram(BackupFile::new(0x8000)),
            gpio: None,
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
    fn parse(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() < 0xC0 {
            return Err("incomplete cartridge header".to_string());
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

        fn clean(s: &str) -> String {
            s.chars().filter(|c| !c.is_ascii_control()).collect()
        }

        let game_title =
            from_utf8(&bytes[0xa0..0xac]).map_err(|_| "invalid game title".to_string())?;
        let game_code =
            from_utf8(&bytes[0xac..0xb0]).map_err(|_| "invalid game code".to_string())?;
        let maker_code =
            from_utf8(&bytes[0xb0..0xb2]).map_err(|_| "invalid marker code".to_string())?;

        println!("{:?}", detect_backup_type(&bytes));

        Ok(Self {
            game_title: clean(game_title),
            game_code: clean(game_code),
            maker_code: clean(maker_code),
            software_version: bytes[0xBC],
            checksum,
        })
    }
}

#[derive(Debug)]
pub enum BackupType {
    Eeprom = 0,
    Sram = 1,
    Flash = 2,
    Flash512 = 3,
    Flash1M = 4,
    AutoDetect = 5,
}

fn detect_backup_type(bytes: &[u8]) -> Option<BackupType> {
    use memmem::*;

    const ID_STRINGS: &'static [&'static str] =
        &["EEPROM", "SRAM", "FLASH_", "FLASH512_", "FLASH1M_"];

    for i in 0..5 {
        let search = TwoWaySearcher::new(ID_STRINGS[i].as_bytes());
        match search.search_in(bytes) {
            Some(_) => {
                return Some(match i {
                    0 => BackupType::Eeprom,
                    1 => BackupType::Sram,
                    2 => BackupType::Flash,
                    3 => BackupType::Flash512,
                    4 => BackupType::Flash1M,
                    5 => BackupType::AutoDetect,
                    _ => unreachable!(),
                })
            }
            _ => {}
        }
    }
    None
}

pub const GPIO_PORT_DATA: u32 = 0xC4;
pub const GPIO_PORT_DIRECTION: u32 = 0xC6;
pub const GPIO_PORT_CONTROL: u32 = 0xC8;

fn is_gpio_access(addr: u32) -> bool {
    match addr & 0x1ff_ffff {
        GPIO_PORT_DATA | GPIO_PORT_DIRECTION | GPIO_PORT_CONTROL => true,
        _ => false,
    }
}

impl Bus for Cartridge {
    fn read_8(&mut self, addr: Addr) -> u8 {
        let offset = (addr & 0x01FF_FFFF) as usize;
        match addr & 0xFF000000 {
            SRAM_LO | SRAM_HI => match &self.backup {
                BackupMedia::Sram(memory) => memory.read((addr & 0x7FFF) as usize),
                _ => todo!(),
            },
            _ => {
                if offset >= self.size {
                    self.read_unused(addr)
                } else {
                    self.bytes[offset as usize]
                }
            }
        }
    }

    fn read_16(&mut self, addr: u32) -> u16 {
        const EEPROM_BASE_ADDR: u32 = 0x0DFF_FF00;

        if is_gpio_access(addr) {
            if let Some(gpio) = &self.gpio {
                todo!()
                //     if !(gpio.is_readable()) {
                //         println!("trying to read GPIO when reads are not allowed");
                //     }
                //     return gpio.read(addr & 0x1ff_ffff);
            }
        }

        if addr & 0xff000000 == GAMEPAK_WS2_HI
            && (self.bytes.len() <= 16 * 1024 * 1024 || addr >= EEPROM_BASE_ADDR)
        {
            if let BackupMedia::_Eeprom(spi) = &self.backup {
                todo!();
                // return spi.read_half(addr);
            }
        }
        self.default_read_16(addr)
    }

    fn write_8(&mut self, addr: Addr, val: u8) {
        match addr & 0xFF000000 {
            SRAM_LO | SRAM_HI => match &mut self.backup {
                BackupMedia::Sram(memory) => memory.write((addr & 0x7FFF) as usize, val),
                _ => todo!(),
            },
            _ => {} // _ => panic!("{:08X} <== {:02X}", addr, val),
        }
    }
}

pub trait BackupMemoryInterface {
    fn write(&mut self, offset: usize, value: u8);
    fn read(&self, offset: usize) -> u8;
    fn resize(&mut self, new_size: usize);
}

#[derive(Clone, Debug)]
enum BackupMedia {
    Sram(BackupFile),
    _Flash(Flash),
    _Eeprom(()),
    _Undetected,
}

#[derive(Clone, Debug)]
struct Flash;

#[derive(Clone, Debug)]
struct BackupFile {
    size: usize,
    buffer: Vec<u8>,
}

impl BackupFile {
    fn new(size: usize) -> Self {
        Self {
            size,
            buffer: vec![0xFF; size],
        }
    }
}

impl BackupMemoryInterface for BackupFile {
    fn write(&mut self, offset: usize, value: u8) {
        self.buffer[offset] = value;
    }

    fn read(&self, offset: usize) -> u8 {
        self.buffer[offset]
    }

    fn resize(&mut self, new_size: usize) {
        self.size = new_size;
        self.buffer.resize(new_size, 0xff);
    }
}
