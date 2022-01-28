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
pub mod sound;
pub mod sysbus;
pub mod timer;
pub mod rtc;

pub trait VideoInterface {
    fn render(&mut self, buffer: &[u8]);
    fn poll(&mut self) -> u16 {
        KEYINPUT_ALL_RELEASED
    }
    fn get_sample_rate(&self) -> i32 {
        44100
    }
    fn push_sample(&mut self, _samples: &[i16]) {}
}

pub trait GpuMemoryMappedIO {
    fn read(&self) -> u16;
    fn write(&mut self, value: u16);
}
