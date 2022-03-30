use enum_dispatch::enum_dispatch;
use rom::Rom;
use sram::Sram;
use std::{
    fs,
    path::{Path, PathBuf},
};

mod rom;

pub struct Gamepak {
    pub rom: Rom,
    _gpio: (),
    save: Saves,
}

impl Gamepak {
    pub fn new() -> Self {
        Self {
            rom: Rom::default(),
            _gpio: (),
            save: Saves::default(),
        }
    }

    pub fn load(&mut self, rom: Option<&Path>, save: Option<&Path>) {
        assert!(rom.is_some() || save.is_some());

        if let Some(path) = rom {
            self.rom.load(path).unwrap();
            let mut path = path.to_path_buf();
            path.set_extension(".sav");
            self.save = Saves::new(self.rom.as_ref(), path)
        }

        if let Some(_path) = save {
            todo!()
        }
    }

    pub fn is_eeprom_access(&self, _addr: u32) -> bool {
        todo!()
    }

    pub fn read_save(&self, addr: u32) -> u8 {
        self.save.read(addr)
    }

    pub fn write_save(&mut self, addr: u32, val: u8) {
        self.save.write(addr, val)
    }
}

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
trait Save {
    fn read(&self, addr: u32) -> u8;
    fn write(&mut self, addr: u32, value: u8);

    fn is_dirty(&mut self) -> bool;
    fn get_save_file(&self) -> &PathBuf;
    fn get_mem(&self) -> &[u8];
}

#[enum_dispatch]
enum Saves {
    Sram,
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
                SaveType::Flash => todo!(),
                SaveType::Flash512 => todo!(),
                SaveType::Flash1M => todo!(),
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

mod sram {
    use super::{Save, Saves};
    use std::path::PathBuf;

    pub struct Sram {
        pub(super) data: Box<[u8]>,
        pub(super) _save_file: PathBuf,
        pub(super) is_dirty: bool,
    }

    impl Sram {
        const SIZE: usize = 0x8000;

        pub fn new(save_file: PathBuf) -> Self {
            Self {
                data: Saves::get_initial_data(&save_file, 0, Self::SIZE),
                _save_file: save_file,
                is_dirty: false,
            }
        }
    }

    impl Save for Sram {
        fn read(&self, addr: u32) -> u8 {
            let addr = addr as usize;
            if addr < Self::SIZE {
                self.data[addr]
            } else {
                0
            }
        }

        fn write(&mut self, addr: u32, value: u8) {
            let addr = addr as usize;
            if addr < Self::SIZE {
                self.is_dirty = true;
                self.data[addr] = value
            }
        }

        fn is_dirty(&mut self) -> bool {
            todo!()
        }

        fn get_save_file(&self) -> &PathBuf {
            todo!()
        }

        fn get_mem(&self) -> &[u8] {
            todo!()
        }
    }
}
