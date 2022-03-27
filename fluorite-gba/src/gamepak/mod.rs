use rom::Rom;
use std::path::Path;

mod rom;

pub struct Gamepak {
    rom: Rom,
    gpio: (),
    save: (),
}

impl Gamepak {
    pub fn new() -> Self {
        Self {
            rom: Rom::default(),
            gpio: (),
            save: (),
        }
    }

    pub fn load(&mut self, rom: Option<&Path>, save: Option<&Path>) {
        assert!(rom.is_some() || save.is_some());

        if let Some(path) = rom {
            self.rom.load(path).unwrap()
        }
    }

    pub fn ready(&self) -> bool {
        !self.rom.data.is_empty()
    }
}
