use std::path::PathBuf;

pub struct Config {
    pub bios_file: PathBuf,
    pub bios_skip: bool,
    pub fast_forward: u32,
    pub frame_size: u32,
    pub volume: f32,
    pub mute: bool,
}

impl Config {
    #[must_use]
    pub fn new() -> Self {
        Self {
            bios_file: "roms/gba_bios.bin".into(),
            bios_skip: true,
            fast_forward: 1000000,
            frame_size: 4,
            volume: 0.5,
            mute: true,
        }
    }
}
