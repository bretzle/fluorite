use std::{fs, path::PathBuf};

use enum_dispatch::enum_dispatch;

use self::flash::Flash;
use self::sram::Sram;

mod flash;
mod sram;

enum SaveType {
    Eeprom = 0,
    Sram = 1,
    Flash = 2,
    Flash512 = 3,
    Flash1M = 4,
}

impl SaveType {
    fn from(value: usize) -> Self {
        use SaveType::*;
        match value {
            0 => Eeprom,
            1 => Sram,
            2 => Flash,
            3 => Flash512,
            4 => Flash1M,
            _ => panic!("Invalid Cart Backup Type!"),
        }
    }
}

#[enum_dispatch(Saves)]
pub trait SaveDevice {
    fn read(&self, addr: u32) -> u8;
    fn write(&mut self, addr: u32, value: u8);

    #[allow(clippy::wrong_self_convention)]
    fn is_dirty(&mut self) -> bool;
    fn get_save_file(&self) -> &PathBuf;
    fn get_mem(&self) -> &[u8];
}

#[enum_dispatch]
pub enum Saves {
    Sram,
    Flash,
}

impl Default for Saves {
    fn default() -> Self {
        Self::Sram(Sram {
            data: Box::new([]),
            _save_file: "".into(),
            is_dirty: false,
        })
    }
}

impl Saves {
    const ID_STRINGS: [&'static [u8]; 5] = [
        "EEPROM_V".as_bytes(),
        "SRAM_V".as_bytes(),
        "FLASH_V".as_bytes(),
        "FLASH512_V".as_bytes(),
        "FLASH1M_V".as_bytes(),
    ];

    pub fn new(rom: &[u8], save_file: PathBuf) -> Self {
        if let Some(save_type) = Self::get_type(rom) {
            match save_type {
                SaveType::Eeprom => todo!(),
                SaveType::Sram => Sram::new(save_file).into(),
                SaveType::Flash => Flash::new(save_file, 0x10000).into(),
                SaveType::Flash512 => Flash::new(save_file, 0x10000).into(),
                SaveType::Flash1M => Flash::new(save_file, 0x20000).into(),
            }
        } else {
            println!("Unable to detect Gamepak save type. Defaulting to SRAM");
            Sram::new(save_file).into()
        }
    }

    fn get_type(rom: &[u8]) -> Option<SaveType> {
        let mut ty = None;

        for rom_start in 0..rom.len() {
            for (id_str_i, id_str) in Self::ID_STRINGS.iter().enumerate() {
                if rom_start + id_str.len() <= rom.len()
                    && rom[rom_start..rom_start + id_str.len()] == **id_str
                {
                    ty = Some(SaveType::from(id_str_i));
                    break;
                }
            }
        }

        ty
    }

    fn get_initial_data(save_file: &PathBuf, default_val: u8, size: usize) -> Box<[u8]> {
        if let Ok(data) = fs::read(save_file) {
            if data.len() == size {
                return data.into_boxed_slice();
            }
        }

        vec![default_val; size].into_boxed_slice()
    }
}
