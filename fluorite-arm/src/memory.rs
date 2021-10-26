use crate::{cpu::Arm7tdmi, Addr};

pub trait MemoryInterface {
    fn load_8(&mut self, addr: Addr) -> u8;
    fn load_16(&mut self, addr: Addr) -> u16;
    fn load_32(&mut self, addr: Addr) -> u32;

    fn store_8(&mut self, addr: Addr, val: u8) -> u8;
    fn store_16(&mut self, addr: Addr, val: u16) -> u16;
    fn store_32(&mut self, addr: Addr, val: u32) -> u32;
}

impl<Memory: MemoryInterface> Arm7tdmi<Memory> {
    pub(crate) fn load_8(&mut self, addr: u32) -> u8 {
        self.bus.load_8(addr)
    }
    pub(crate) fn load_16(&mut self, addr: u32) -> u16 {
        self.bus.load_16(addr & !1)
    }
    pub(crate) fn load_32(&mut self, addr: u32) -> u32 {
        self.bus.load_32(addr & !3)
    }

    pub(crate) fn store_8(&mut self, addr: u32, value: u8) {
        self.bus.store_8(addr, value);
    }
    pub(crate) fn store_16(&mut self, addr: u32, value: u16) {
        self.bus.store_16(addr & !1, value);
    }
    pub(crate) fn store_32(&mut self, addr: u32, value: u32) {
        self.bus.store_32(addr & !3, value);
    }
}
