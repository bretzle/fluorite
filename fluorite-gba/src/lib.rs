#![feature(adt_const_params)]
#![allow(clippy::new_without_default, clippy::only_used_in_recursion)]

#[macro_use]
extern crate log;
extern crate num_traits as num;

pub mod arm;
pub mod consts;
pub mod gba;
pub mod io;

pub(crate) static BIOS: &[u8] = include_bytes!("../../roms/gba_bios.bin");

pub trait AudioInterface {
    fn write(&mut self, samples: [i16; 2]);
}
