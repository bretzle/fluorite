use self::{
    gpio::{Gpio, GpioDevice},
    save::{SaveDevice, Saves},
};
use rom::Rom;
use std::path::Path;

pub mod gpio;
mod rom;
mod save;

pub struct Gamepak {
    pub rom: Rom,
    pub gpio: Gpio,
    save: Saves,
}

impl Gamepak {
    pub fn new() -> Self {
        Self {
            rom: Rom::default(),
            gpio: Gpio::default(),
            save: Saves::default(),
        }
    }

    pub fn load(&mut self, rom: Option<&Path>, save: Option<&Path>) {
        assert!(rom.is_some() || save.is_some());

        if let Some(path) = rom {
            self.rom.load(path).unwrap();
            let mut path = path.to_path_buf();
            path.set_extension(".sav");
            self.save = Saves::new(self.rom.as_ref(), path);
            self.gpio = Gpio::new(self.rom.as_ref());
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

    pub fn is_rtc_used(&self) -> bool {
        #[allow(irrefutable_let_patterns)] // TODO: remove this when more gpio things are
        if let Gpio::Rtc(rtc) = &self.gpio {
            return rtc.is_used();
        }

        false
    }

    pub fn is_eeprom(&self) -> bool {
        match self.save {
            Saves::Sram(_) => false,
            Saves::Flash(_) => false,
        }
    }
}
