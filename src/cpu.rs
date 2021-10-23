use std::any::Any;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use fluorite_arm::cpu::Arm7tdmi;
use fluorite_common::Shared;

static RUNNING: AtomicBool = AtomicBool::new(true);
static TRACE_INSTRUCTIONS: AtomicBool = AtomicBool::new(true);

pub struct GbaCpu {
    arm: Arm7tdmi,

    // event_queue: Mutex<VecDeque<ThreadEvent>>,
    scheduler: Shared<()>,
}

impl GbaCpu {
    pub fn new(arm: Arm7tdmi) -> Self {
        Self {
            arm,
            // event_queue: Mutex::default(),
            scheduler: Shared::default(),
        }
    }

    pub fn run(&mut self) {}
}
