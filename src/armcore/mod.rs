use crate::armcore::registers::{CpuMode, CpuState, StatusRegister};
use fluorite_common::Shared;

mod registers;
mod thumb;

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

    pub fn cycle(&mut self) {
        let code = self.pipeline[2];

        if self.pipeline_pos == 3 {
            match self.cspr.state() {
                CpuState::ARM => {
                    // check the condition
                    // run the instruction
                    todo!()
                }
                CpuState::THUMB => {
                    let index = code >> 6;
                    let inst = &thumb::THUMB_LUT[index as usize];
                    inst.execute(self, code as u16);
                }
            }
        } else {
            self.pipeline_pos += 1;
        }

        match self.cspr.state() {
            CpuState::THUMB => {
                if self.increment_pc {
                    self.pc += 2;
                }
                self.increment_pc = true;

                self.pipeline[2] = self.pipeline[1];
                self.pipeline[1] = self.pipeline[0];
                // self.pipeline[0] = self._bus.read_16(self.pc);
            }
            CpuState::ARM => {
                if self.increment_pc {
                    self.pc += 4;
                }
                self.increment_pc = true;

                self.pipeline[2] = self.pipeline[1];
                self.pipeline[1] = self.pipeline[0];
                // self.pipeline[0] = self._bus.read_32(self.pc);
            }
        }
    }

    pub fn stage(&self) -> usize {
        self.pipeline_pos
    }

    pub fn pc_thumb(&self) -> u32 {
        self.pc.wrapping_sub(4)
    }

    pub fn pc_arm(&self) -> u32 {
        self.pc.wrapping_sub(8)
    }
}

// static
