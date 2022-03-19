use self::debug::DebugSpecification;
use crate::gba::{DebugSpec, Pixels, HEIGHT, WIDTH};
use std::sync::{Arc, Mutex};

pub mod debug;

pub struct Gpu {}

impl Gpu {
    pub fn new() -> (Self, Pixels, DebugSpec) {
        let pixels = Arc::new(Mutex::new(vec![0; WIDTH * HEIGHT]));
        let debug = Arc::new(Mutex::new(DebugSpecification::new()));

        let gpu = Self {};

        (gpu, pixels, debug)
    }
}
