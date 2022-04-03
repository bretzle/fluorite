use fluorite_common::EasyLazy;
use std::{cell::Cell, path::PathBuf};

pub static CONFIG: EasyLazy<Config> = EasyLazy::new(Config::new);

pub struct Config {
    pub bios_file: PathBuf,
    pub bios_skip: bool,
    pub fast_forward: u32,
    pub frame_size: u32,
    pub volume: Cell<f32>,
    pub mute: Cell<bool>,
}

impl Config {
    #[must_use]
    pub fn new() -> Self {
        Self {
            bios_file: "roms/gba_bios.bin".into(),
            bios_skip: true,
            fast_forward: 1000000,
            frame_size: 4,
            volume: Cell::new(0.5),
            mute: Cell::new(true),
        }
    }
}
