use crate::{gpu::Gpu, sysbus::Bus};
use fluorite_arm::Addr;

pub const IO_BASE: Addr = 0x0400_0000;
pub const REG_DISPCNT: Addr = 0x0400_0000; // R/W  LCD Control

pub struct IoDevices {
    pub gpu: Gpu,
}

impl IoDevices {
    pub fn new() -> Self {
        Self { gpu: Gpu::new() }
    }
}

impl Bus for IoDevices {
    fn read_8(&mut self, addr: Addr) -> u8 {
        todo!()
    }

    fn read_16(&mut self, addr: Addr) -> u16 {
        todo!()
    }

    fn write_8(&mut self, addr: Addr, val: u8) {
        todo!()
    }

    fn write_16(&mut self, addr: Addr, val: u16) {
        let io_addr = addr + IO_BASE;

        match io_addr {
            REG_DISPCNT => self.gpu.write_dispcnt(val),
            _ => panic!(
                "Unimplemented write to 0x{:08X} {}",
                io_addr,
                io_reg_string(io_addr)
            ),
        }
    }
}

const fn io_reg_string(addr: Addr) -> &'static str {
    match addr {
        REG_DISPCNT => "REG_DISPCNT",
        _ => "UNKNOWN",
    }
}
