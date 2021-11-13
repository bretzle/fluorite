use std::{cell::Cell, rc::Rc};

use crate::{consts::*, gpu::Gpu, interrupt::InterruptController, sysbus::Bus, GpuMemoryMappedIO};
use fluorite_arm::Addr;
use modular_bitfield::{bitfield, prelude::B2};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HaltState {
    Running,
    Halt, // In Halt mode, the CPU is paused as long as (IE AND IF)=0,
    Stop, // In Stop mode, most of the hardware including sound and video are paused
}

pub struct IoDevices {
    pub intc: InterruptController,
    pub dmac: DmaController,
    pub gpu: Gpu,
    pub haltcnt: HaltState,
    pub waitcnt: WaitControl,
}

impl IoDevices {
    pub fn new(gpu: Gpu) -> Self {
        Self {
            gpu,
            intc: InterruptController::new(),
            dmac: DmaController::new(),
            haltcnt: HaltState::Running,
            waitcnt: WaitControl::new(),
        }
    }
}

static_assertions::assert_eq_size!(WaitControl, u16);
#[bitfield]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WaitControl {
    pub sram_wait_control: B2,
    pub ws0_first_access: B2,
    pub ws0_second_access: bool,
    pub ws1_first_access: B2,
    pub ws1_second_access: bool,
    pub ws2_first_access: B2,
    pub ws2_second_access: bool,
    phi_terminal_output: B2,
    prefetch: bool,
    #[skip]
    _reserved: B2,
}

impl Default for WaitControl {
    fn default() -> Self {
        WaitControl::new()
    }
}

pub struct DmaController {}
impl DmaController {
    pub fn new() -> Self {
        Self {}
    }

    pub fn is_active(&self) -> bool {
        // TODO
        false
    }
}

impl Bus for IoDevices {
    fn read_8(&mut self, addr: Addr) -> u8 {
        let t = self.read_16(addr & !1);
        if addr & 1 != 0 {
            (t >> 8) as u8
        } else {
            t as u8
        }
    }

    fn read_16(&mut self, addr: Addr) -> u16 {
        let io_addr = addr + IO_BASE;

        match io_addr {
            REG_DISPCNT => self.gpu.dispcnt.read(),
            REG_DISPSTAT => self.gpu.dispstat.into(),
            REG_VCOUNT => self.gpu.vcount as u16,
            _ => {
                panic!(
                    "Unimplemented read from 0x{:08X} {}",
                    io_addr,
                    io_reg_string(io_addr)
                )
            }
        }
    }

    fn write_8(&mut self, addr: Addr, val: u8) {
        todo!()
    }

    fn write_16(&mut self, addr: Addr, val: u16) {
        let io_addr = addr + IO_BASE;

        match io_addr {
            REG_DISPCNT => self.gpu.write_dispcnt(val),
            REG_HALTCNT => {
                if val & 0x80 != 0 {
                    self.haltcnt = HaltState::Stop;
                    panic!("Can't handle HaltCtrl == Stop yet");
                } else {
                    self.haltcnt = HaltState::Halt;
                }
            }
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
        REG_DISPSTAT => "REG_DISPSTAT",
        REG_VCOUNT => "REG_VCOUNT",
        _ => "UNKNOWN",
    }
}
