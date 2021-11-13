use crate::{cpu::Arm7tdmi, Addr};

pub trait MemoryInterface {
    fn load_8(&mut self, addr: Addr) -> u8;
    fn load_16(&mut self, addr: Addr) -> u16;
    fn load_32(&mut self, addr: Addr) -> u32;

    fn store_8(&mut self, addr: Addr, val: u8);
    fn store_16(&mut self, addr: Addr, val: u16);
    fn store_32(&mut self, addr: Addr, val: u32);

    fn idle_cycle(&mut self);
}

#[allow(dead_code)]
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

    pub(crate) fn store_aligned_32(&mut self, addr: Addr, value: u32) {
        self.store_32(addr & !0x3, value);
    }

    pub(crate) fn store_aligned_16(&mut self, addr: Addr, value: u16) {
        self.store_16(addr & !0x1, value);
    }

    /// Helper function for "ldr" instruction that handles misaligned addresses
    pub(crate) fn ldr_word(&mut self, addr: Addr) -> u32 {
        if addr & 0x3 != 0 {
            let rotation = (addr & 0x3) << 3;
            let value = self.load_32(addr & !0x3);
            let mut carry = self.cspr.c();
            let v = self.ror(value, rotation, &mut carry, false, false);
            self.cspr.set_c(carry);
            v
        } else {
            self.load_32(addr)
        }
    }

    /// Helper function for "ldrh" instruction that handles misaligned addresses
    pub(crate) fn ldr_half(&mut self, addr: Addr) -> u32 {
        if addr & 0x1 != 0 {
            let rotation = (addr & 0x1) << 3;
            let value = self.load_16(addr & !0x1);
            let mut carry = self.cspr.c();
            let v = self.ror(value as u32, rotation, &mut carry, false, false);
            self.cspr.set_c(carry);
            v
        } else {
            self.load_16(addr) as u32
        }
    }

    /// Helper function for "ldrsh" instruction that handles misaligned addresses
    pub(crate) fn ldr_sign_half(&mut self, addr: Addr) -> u32 {
        if addr & 0x1 != 0 {
            self.load_8(addr) as i8 as i32 as u32
        } else {
            self.load_16(addr) as i16 as i32 as u32
        }
    }

    pub(crate) fn idle_cycle(&mut self) {
        self.bus.idle_cycle()
    }
}
