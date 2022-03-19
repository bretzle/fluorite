mod arm;
pub mod registers;
mod thumb;

use std::mem::size_of;

use num::cast;

use self::registers::Registers;
use crate::io::{memory::MemoryValue, MemoryAccess, Sysbus};

include!(concat!(env!("OUT_DIR"), "/cond_lut.rs"));

pub(self) type InstructionHandler<T> = fn(&mut Arm7tdmi, &mut Sysbus, T);

pub struct Arm7tdmi {
    pub regs: Registers,
    pipeline: [u32; 2],
    next_access: MemoryAccess,
    internal: bool,
}

impl Arm7tdmi {
    pub fn new(bios: bool, bus: &mut Sysbus) -> Self {
        let mut arm = Self {
            regs: if bios { Registers::new() } else { todo!() },
            pipeline: [0; 2],
            next_access: MemoryAccess::N,
            internal: false,
        };
        arm.fill_arm_instr_buffer(bus);
        arm
    }

    pub fn emulate_instr(&mut self, bus: &mut Sysbus) {
        if self.regs.get_t() {
            // TODO
        } else {
            self.emulate_arm_instr(bus);
        }
    }

    pub fn read<T>(&mut self, bus: &mut Sysbus, access: MemoryAccess, addr: u32) -> T
    where
        T: MemoryValue,
    {
        bus.setup_openbus(self.regs.pc, self.regs.get_t(), &self.pipeline);
        let val = bus.read(addr);
        bus.inc_clock(
            self.next_access,
            addr,
            match size_of::<T>() {
                1 => 0,
                2 => 1,
                4 => 2,
                _ => unreachable!(),
            },
        );
        self.next_access = access;
        val
    }

    pub fn write<T>(&mut self, bus: &mut Sysbus, access: MemoryAccess, addr: u32, value: T)
    where
        T: MemoryValue,
    {
        bus.setup_openbus(self.regs.pc, self.regs.get_t(), &self.pipeline);
        bus.inc_clock(
            self.next_access,
            addr,
            match size_of::<T>() {
                1 => 0,
                2 => 1,
                4 => 2,
                _ => unreachable!(),
            },
        );
        self.next_access = access;
        bus.write(addr, value);
    }

    pub fn instruction_prefetch<T>(&mut self, bus: &mut Sysbus, access: MemoryAccess)
    where
        T: MemoryValue,
    {
        self.pipeline[1] = cast::<T, u32>(self.read(bus, access, self.regs.pc)).unwrap();
        self.internal = false;
    }

    pub fn handle_irq(&mut self, bus: &mut Sysbus) {
        // TODO
    }
}
