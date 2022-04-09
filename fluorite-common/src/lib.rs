#![feature(generic_const_exprs)]
#![feature(once_cell)]
#![allow(clippy::mut_from_ref, incomplete_features, clippy::missing_safety_doc)]

pub extern crate flume;
pub extern crate static_assertions;

mod bits;
mod cell;
mod mem;
pub mod traits;

pub use bits::*;
pub use cell::*;
pub use mem::*;

pub use fluorite_macros::*;
