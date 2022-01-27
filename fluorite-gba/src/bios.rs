use fluorite_arm::{Addr, cpu::Arm7tdmi};
use fluorite_common::WeakPointer;

use crate::sysbus::{Bus, SysBus};

#[derive(Clone)]
pub struct Bios {
    rom: Box<[u8]>,
    last_opcode: u32,
    arm_core: WeakPointer<Arm7tdmi<SysBus>>,
}

impl Bios {
    pub fn new(bios: &[u8]) -> Self {
        Self {
            rom: bios.to_vec().into_boxed_slice(),
            last_opcode: 0xE129F000,
            arm_core: WeakPointer::default(),
        }
    }

    // TODO: does this need to be public
    pub fn connect_arm_core(&mut self, arm_ptr: WeakPointer<Arm7tdmi<SysBus>>) {
        self.arm_core = arm_ptr;
    }

    // TODO: check if read is allowed
    fn read_allowed(&self) -> bool {
        self.arm_core.pc < 0x4000
    }
}

impl Bus for Bios {
    fn read_8(&mut self, addr: Addr) -> u8 {
        if self.read_allowed() {
            self.rom.read_8(addr)
        } else {
            (self.last_opcode >> ((addr & 3) << 3)) as u8
        }
    }

    fn read_16(&mut self, addr: Addr) -> u16 {
        if self.read_allowed() {
            self.rom.read_16(addr) as u16
        } else {
            (self.last_opcode >> ((addr & 2) << 3)) as u16
        }
    }

    fn read_32(&mut self, addr: Addr) -> u32 {
        if self.read_allowed() {
            let val = self.rom.read_32(addr);
            self.last_opcode = val;
            val
        } else {
            self.last_opcode
        }
    }

    fn write_8(&mut self, _addr: Addr, _val: u8) {}
    fn write_16(&mut self, _addr: Addr, _val: u16) {}
    fn write_32(&mut self, _addr: Addr, _val: u32) {}
}
