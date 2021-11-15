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

    // TODO: check if read is allowed
    fn read_allowed(&self) -> bool {
        true
    }
}

impl Bus for Bios {
    fn read_8(&mut self, _addr: Addr) -> u8 {
        todo!()
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
