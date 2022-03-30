use super::{SaveDevice, Saves};
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

impl SaveDevice for Sram {
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
