#![doc(html_logo_url = "https://raw.githubusercontent.com/bretzle/fluorite/main/fluorite.png")]
#![allow(clippy::identity_op)]

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
pub mod sched;
pub mod sysbus;

pub trait VideoInterface {
    fn render(&mut self, buffer: &[u8]);
}

pub trait GpuMemoryMappedIO {
    fn read(&self) -> u16;
    fn write(&mut self, value: u16);
}
