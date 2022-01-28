use crate::{consts::*, rtc::Rtc, sysbus::Bus};
use bitfield::Bit;
use fluorite_arm::Addr;
use std::{fs::File, path::PathBuf, str::from_utf8};

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
        let save_type = detect_backup_type(bytes);

        println!("{:?}", save_type);
        println!("{:#?}", header);

        let mut backup = BackupMedia::Undetected;
        if let Some(backup_type) = save_type {
            backup = match backup_type {
                BackupType::Flash => todo!(),
                BackupType::Flash512 => todo!(),
                BackupType::Flash1M => BackupMedia::Flash(Flash::new(None, FlashSize::Flash128k)),
                BackupType::Sram => BackupMedia::Sram(BackupFile::new(0x8000, None)),
                BackupType::Eeprom => todo!(),
                BackupType::AutoDetect => todo!(),
            }
        }

        Ok(Self {
            header,
            bytes: bytes.to_vec().into_boxed_slice(),
            size,
            backup,
            gpio: Some(Gpio::new(Some(Rtc::new()))), // TODO: dont force using rtc
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
pub const EEPROM_BASE_ADDR: u32 = 0x0DFF_FF00;

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
                BackupMedia::Flash(flash) => flash.read(addr),
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
        if is_gpio_access(addr) {
            if let Some(gpio) = &self.gpio {
                if !(gpio.is_readable()) {
                    println!("trying to read GPIO when reads are not allowed");
                }
                return gpio.read(addr & 0x1ff_ffff);
            }
        }

        if addr & 0xff000000 == GAMEPAK_WS2_HI
            && (self.bytes.len() <= 16 * 1024 * 1024 || addr >= EEPROM_BASE_ADDR)
        {
            if let BackupMedia::Eeprom(spi) = &self.backup {
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
                BackupMedia::Flash(flash) => flash.write(addr, val),
                _ => todo!(),
            },
            _ => {} // _ => panic!("{:08X} <== {:02X}", addr, val),
        }
    }

    fn write_16(&mut self, addr: u32, value: u16) {
        if is_gpio_access(addr) {
            if let Some(gpio) = &mut self.gpio {
                gpio.write(addr & 0x1ff_ffff, value);
                return;
            }
        }

        if addr & 0xff000000 == GAMEPAK_WS2_HI
            && (self.bytes.len() <= 16 * 1024 * 1024 || addr >= EEPROM_BASE_ADDR)
        {
            if let BackupMedia::Eeprom(spi) = &mut self.backup {
                todo!()
            }
        }

        self.default_write_16(addr, value);
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
    Flash(Flash),
    Eeprom(()),
    Undetected,
}

#[derive(Clone, Debug)]
pub struct Flash {
    chip_id: u16,
    size: usize,
    wrseq: FlashWriteSequence,
    mode: FlashMode,
    bank: usize,

    memory: BackupFile,
}

const MACRONIX_64K_CHIP_ID: u16 = 0x1CC2;
const MACRONIX_128K_CHIP_ID: u16 = 0x09c2;

const SECTOR_SIZE: usize = 0x1000;
const BANK_SIZE: usize = 0x10000;

impl Flash {
    pub fn new(flash_path: Option<PathBuf>, flash_size: FlashSize) -> Flash {
        let chip_id = match flash_size {
            FlashSize::Flash64k => MACRONIX_64K_CHIP_ID,
            FlashSize::Flash128k => MACRONIX_128K_CHIP_ID,
        };

        let size: usize = flash_size.into();
        let memory = BackupFile::new(size, flash_path);

        Flash {
            chip_id: chip_id,
            wrseq: FlashWriteSequence::Initial,
            mode: FlashMode::Initial,
            size: size,
            bank: 0,
            memory: memory,
        }
    }

    fn reset_sequence(&mut self) {
        self.wrseq = FlashWriteSequence::Initial;
    }

    fn command(&mut self, addr: u32, value: u8) {
        const COMMAND_ADDR: u32 = 0x0E00_5555;

        if let Some(command) = FlashCommand::from_u8(value) {
            match (addr, command) {
                (COMMAND_ADDR, FlashCommand::EnterIdMode) => {
                    self.mode = FlashMode::ChipId;
                    self.reset_sequence();
                }
                (COMMAND_ADDR, FlashCommand::TerminateIdMode) => {
                    self.mode = FlashMode::Initial;
                    self.reset_sequence();
                }
                (COMMAND_ADDR, FlashCommand::Erase) => {
                    self.mode = FlashMode::Erase;
                    self.reset_sequence();
                }
                (COMMAND_ADDR, FlashCommand::EraseEntireChip) => {
                    if self.mode == FlashMode::Erase {
                        for i in 0..self.size {
                            self.memory.write(i, 0xff);
                        }
                    }
                    self.reset_sequence();
                    self.mode = FlashMode::Initial;
                }
                (sector_n, FlashCommand::EraseSector) => {
                    let sector_offset = self.flash_offset((sector_n & 0xf000) as usize);

                    for i in 0..SECTOR_SIZE {
                        self.memory.write(sector_offset + i, 0xff);
                    }
                    self.reset_sequence();
                    self.mode = FlashMode::Initial;
                }
                (COMMAND_ADDR, FlashCommand::WriteByte) => {
                    self.mode = FlashMode::Write;
                    self.wrseq = FlashWriteSequence::Argument;
                }
                (COMMAND_ADDR, FlashCommand::SelectBank) => {
                    self.mode = FlashMode::Select;
                    self.wrseq = FlashWriteSequence::Argument;
                }
                (addr, command) => {
                    panic!("[FLASH] Invalid command {:?} addr {:#x}", command, addr);
                }
            };
        } else {
            panic!("[FLASH] unknown command {:x}", value);
        }
    }

    /// Returns the phyiscal offset inside the flash file according to the selected bank
    #[inline]
    fn flash_offset(&self, offset: usize) -> usize {
        let offset = (offset & 0xffff) as usize;
        return self.bank * BANK_SIZE + offset;
    }

    pub fn read(&self, addr: u32) -> u8 {
        let offset = (addr & 0xffff) as usize;
        let result = if self.mode == FlashMode::ChipId {
            match offset {
                0 => (self.chip_id & 0xff) as u8,
                1 => (self.chip_id >> 8) as u8,
                _ => panic!("Tried to read invalid flash offset while reading chip ID"),
            }
        } else {
            self.memory.read(self.flash_offset(offset))
        };

        result
    }

    pub fn write(&mut self, addr: Addr, value: u8) {
        println!("[FLASH] write {:#x}={:#x}", addr, value);
        match self.wrseq {
            FlashWriteSequence::Initial => {
                if addr == 0x0E00_5555 && value == 0xAA {
                    self.wrseq = FlashWriteSequence::Magic;
                }
            }
            FlashWriteSequence::Magic => {
                if addr == 0xE00_2AAA && value == 0x55 {
                    self.wrseq = FlashWriteSequence::Command;
                }
            }
            FlashWriteSequence::Command => {
                self.command(addr, value);
            }
            FlashWriteSequence::Argument => {
                match self.mode {
                    FlashMode::Write => {
                        self.memory
                            .write(self.flash_offset((addr & 0xffff) as usize), value);
                    }
                    FlashMode::Select => {
                        if addr == 0x0E00_0000 {
                            self.bank = value as usize;
                        }
                    }
                    _ => panic!("Flash sequence is invalid"),
                };
                self.mode = FlashMode::Initial;
                self.reset_sequence();
            }
        }
    }
}

#[derive(Debug)]
enum FlashCommand {
    EnterIdMode = 0x90,
    TerminateIdMode = 0xf0,
    Erase = 0x80,
    EraseEntireChip = 0x10,
    EraseSector = 0x30,
    WriteByte = 0xa0,
    SelectBank = 0xb0,
}

impl FlashCommand {
    fn from_u8(val: u8) -> Option<Self> {
        Some(match val {
            0x90 => FlashCommand::EnterIdMode,
            0xf0 => FlashCommand::TerminateIdMode,
            0x80 => FlashCommand::Erase,
            0x10 => FlashCommand::EraseEntireChip,
            0x30 => FlashCommand::EraseSector,
            0xa0 => FlashCommand::WriteByte,
            0xb0 => FlashCommand::SelectBank,
            _ => return None,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
enum FlashMode {
    Initial,
    ChipId,
    Erase,
    Write,
    Select,
}

#[derive(Clone, Debug)]
enum FlashWriteSequence {
    Initial,
    Magic,
    Command,
    Argument,
}

#[derive(Debug)]
pub enum FlashSize {
    Flash64k,
    Flash128k,
}

impl Into<usize> for FlashSize {
    fn into(self) -> usize {
        match self {
            FlashSize::Flash64k => 64 * 1024,
            FlashSize::Flash128k => 128 * 1024,
        }
    }
}

#[derive(Debug)]
pub struct BackupFile {
    size: usize,
    path: Option<PathBuf>,
    file: Option<File>,
    buffer: Vec<u8>,
}

impl BackupFile {
    pub fn new(size: usize, path: Option<PathBuf>) -> Self {
        let mut file = None;
        let buffer = if let Some(path) = &path {
            todo!()
        } else {
            vec![0xff; size]
        };

        Self {
            size,
            path,
            file,
            buffer,
        }
    }
}

impl Clone for BackupFile {
    fn clone(&self) -> Self {
        BackupFile::new(self.size, self.path.clone())
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

////////////////////////////////////////

pub type GpioState = [GpioDirection; 4];

#[derive(Debug, Clone, PartialEq, Eq)]
enum GpioPortControl {
    WriteOnly,
    ReadWrite,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GpioDirection {
    In,
    Out,
}

#[derive(Debug, Clone)]
pub struct Gpio {
    pub(crate) rtc: Option<Rtc>,
    direction: GpioState,
    control: GpioPortControl,
}

impl Gpio {
    pub fn new(rtc: Option<Rtc>) -> Self {
        Self {
            rtc,
            direction: [GpioDirection::Out; 4],
            control: GpioPortControl::WriteOnly,
        }
    }

    pub fn is_readable(&self) -> bool {
        self.control != GpioPortControl::WriteOnly
    }

    pub fn read(&self, addr: Addr) -> u16 {
        match addr {
            GPIO_PORT_DATA => {
                if let Some(rtc) = &self.rtc {
                    rtc.read(&self.direction)
                } else {
                    0
                }
            }
            GPIO_PORT_DIRECTION => todo!(),
            GPIO_PORT_CONTROL => todo!(),
            _ => unreachable!("{:?}", addr),
        }
    }

    pub fn write(&mut self, addr: Addr, value: u16) {
        match addr {
            GPIO_PORT_DATA => {
                if let Some(rtc) = &mut self.rtc {
                    rtc.write(&self.direction, value);
                }
            }
            GPIO_PORT_DIRECTION => {
                for idx in 0..4 {
                    self.direction[idx] = if value.bit(idx) {
                        GpioDirection::Out
                    } else {
                        GpioDirection::In
                    }
                }
            }
            GPIO_PORT_CONTROL => {
                self.control = if value != 0 {
                    GpioPortControl::ReadWrite
                } else {
                    GpioPortControl::WriteOnly
                }
            }
            _ => unreachable!("{:?}", addr),
        }
    }
}

pub trait GpioDevice: Sized {
    fn write(&mut self, gpio_state: &GpioState, data: u16);
    fn read(&self, gpio_state: &GpioState) -> u16;
}
