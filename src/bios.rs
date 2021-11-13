use fluorite_arm::Addr;

use crate::sysbus::Bus;

#[derive(Clone)]
pub struct Bios {
    rom: Box<[u8]>,
    last_opcode: u32,
}

impl Bios {
    pub fn new(bios: &[u8]) -> Self {
        Self {
            rom: bios.to_vec().into_boxed_slice(),
            last_opcode: 0xE129F000,
        }
    }
}

impl Bus for Bios {
    fn read_8(&mut self, addr: Addr) -> u8 {
        todo!()
    }

    fn read_16(&mut self, addr: Addr) -> u16 {
        todo!()
    }

    fn read_32(&mut self, addr: Addr) -> u32 {
        // TODO: check if read is allowed
        if true {
            let val = self.rom.read_32(addr);
            self.last_opcode = val;
            val
        } else {
            self.last_opcode
        }
    }

    fn write_8(&mut self, addr: Addr, val: u8) {}
    fn write_16(&mut self, addr: Addr, val: u16) {}
    fn write_32(&mut self, addr: Addr, val: u32) {}
}