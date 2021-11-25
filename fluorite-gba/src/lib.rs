#![doc(html_logo_url = "https://raw.githubusercontent.com/bretzle/fluorite/main/fluorite.png")]
#![allow(clippy::identity_op)]

use keypad::KEYINPUT_ALL_RELEASED;

#[macro_use]
extern crate fluorite_common;

pub mod bios;
pub mod cartridge;
pub mod consts;
pub mod dma;
pub mod gba;
pub mod gpu;
pub mod interrupt;
pub mod iodev;
pub mod keypad;
pub mod sched;
pub mod sysbus;
pub mod timer;

pub trait VideoInterface {
    fn render(&mut self, buffer: &[u8]);
    fn poll(&mut self) -> u16 {
        KEYINPUT_ALL_RELEASED
    }
}

pub trait GpuMemoryMappedIO {
    fn read(&self) -> u16;
    fn write(&mut self, value: u16);
}
