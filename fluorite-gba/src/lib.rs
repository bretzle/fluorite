#![allow(clippy::new_without_default)]

extern crate num_traits as num;

pub mod arm;
pub mod consts;
pub mod gba;
pub mod io;

pub trait AudioInterface {
    fn write(&mut self, samples: [i16; 2]);
}
