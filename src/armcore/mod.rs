use crate::armcore::registers::{CpuMode, StatusRegister};
use crate::util::Shared;

mod registers;

pub struct Arm7tdmi {
    _bus: Shared<()>,

    // registers
    pc: u32,
    gpr: [u32; 15],
    cspr: StatusRegister,
    _spsr: (),
    _banked: (),

    // pipelining
    pipeline: [u32; 3],
    pipeline_pos: usize,

    increment_pc: bool,

    _options: (),
}

impl Arm7tdmi {
    pub fn new(_bus: Shared<()>) -> Self {
        Self {
            _bus,
            pc: 0,
            gpr: [0; 15],
            cspr: StatusRegister::new().with_mode(CpuMode::System),
            _spsr: (),
            _banked: (),
            pipeline: [0; 3],
            pipeline_pos: 0,
            increment_pc: false,
            _options: (),
        }
    }
}
