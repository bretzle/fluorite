use std::cmp;

use crate::{
    consts::*,
    dma::DmaController,
    gpu::{Gpu, WindowFlags},
    interrupt::InterruptController,
    sysbus::{Bus, SysBus},
    GpuMemoryMappedIO,
};
use fluorite_arm::Addr;
use fluorite_common::WeakPointer;
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
    pub post_boot_flag: bool,

    sysbus_ptr: WeakPointer<SysBus>,
}

impl IoDevices {
    pub fn new(gpu: Gpu, dmac: DmaController) -> Self {
        Self {
            gpu,
            intc: InterruptController::new(),
            dmac,
            haltcnt: HaltState::Running,
            waitcnt: WaitControl::new(),
            post_boot_flag: false,
            sysbus_ptr: Default::default(),
        }
    }

    pub fn set_sysbus_ptr(&mut self, ptr: WeakPointer<SysBus>) {
        self.sysbus_ptr = ptr;
    }
}

static_assertions::assert_eq_size!(WaitControl, u16);
#[bitfield]
#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WaitControl {
    pub sram_wait_control: B2,
    pub ws0_first_access: B2,
    pub ws0_second_access: bool,
    pub ws1_first_access: B2,
    pub ws1_second_access: bool,
    pub ws2_first_access: B2,
    pub ws2_second_access: bool,
    #[skip]
    phi_terminal_output: B2,
    #[skip]
    prefetch: bool,
    #[skip]
    _reserved: B2,
}

impl Default for WaitControl {
    fn default() -> Self {
        WaitControl::new()
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
            REG_DISPSTAT => self.gpu.dispstat.read(),
            REG_VCOUNT => self.gpu.vcount as u16,
            REG_BG0CNT => self.gpu.bgcnt[0].read(),
            REG_BG1CNT => self.gpu.bgcnt[1].read(),
            REG_BG2CNT => self.gpu.bgcnt[2].read(),
            REG_BG3CNT => self.gpu.bgcnt[3].read(),
            REG_IME => self.intc.master_enable as u16,
            REG_POSTFLG => self.post_boot_flag as u16,
            _ => {
                let s = io_reg_string(io_addr);

                match s {
                    "UNKNOWN" => {
                        println!("Unimplemented read from 0x{:08X} {}", io_addr, s);
                        0
                    }
                    _ => {
                        panic!("Unimplemented read from 0x{:08X} {}", io_addr, s);
                    }
                }
            }
        }
    }

    fn write_8(&mut self, addr: Addr, val: u8) {
        match addr + IO_BASE {
            /* FIFO_A */
            0x0400_00A0 | 0x0400_00A1 | 0x0400_00A2 | 0x0400_00A3 => {
                todo!()
                // self.sound.write_fifo(0, val as i8)
            }
            /* FIFO_B */
            0x0400_00A4 | 0x0400_00A5 | 0x0400_00A6 | 0x0400_00A7 => {
                todo!()
                // self.sound.write_fifo(1, val as i8)
            }
            _ => {
                let t = self.read_16(addr & !1);
                let t = if addr & 1 != 0 {
                    (t & 0xff) | (val as u16) << 8
                } else {
                    (t & 0xff00) | (val as u16)
                };
                self.write_16(addr & !1, t);
            }
        }
    }

    fn write_16(&mut self, addr: Addr, val: u16) {
        let mut io = self;
        let io_addr = addr + IO_BASE;

        macro_rules! write_reference_point {
            (low bg $coord:ident $internal:ident) => {{
                let i = ((io_addr - REG_BG2X_L) / 0x10) as usize;
                let t = io.gpu.bg_aff[i].$coord as u32;
                io.gpu.bg_aff[i].$coord = ((t & 0xffff0000) + (val as u32)) as i32;
                let new_value = ((t & 0xffff0000) + (val as u32)) as i32;
                io.gpu.bg_aff[i].$coord = new_value;
                io.gpu.bg_aff[i].$internal = new_value;
            }};
            (high bg $coord:ident $internal:ident) => {{
                let i = ((io_addr - REG_BG2X_L) / 0x10) as usize;
                let t = io.gpu.bg_aff[i].$coord;
                let new_value = (t & 0xffff) | ((sign_extend_i32((val & 0xfff) as i32, 12)) << 16);
                io.gpu.bg_aff[i].$coord = new_value;
                io.gpu.bg_aff[i].$internal = new_value;
            }};
        }

        match io_addr {
            REG_DISPCNT => io.gpu.write_dispcnt(val),
            REG_DISPSTAT => io.gpu.dispstat.write(val),
            REG_VCOUNT => io.gpu.vcount = val as usize,
            REG_BG0CNT => io.gpu.bgcnt[0].write(val),
            REG_BG1CNT => io.gpu.bgcnt[1].write(val),
            REG_BG2CNT => io.gpu.bgcnt[2].write(val),
            REG_BG3CNT => io.gpu.bgcnt[3].write(val),
            REG_BG0HOFS => io.gpu.bg_hofs[0] = val & 0x1ff,
            REG_BG0VOFS => io.gpu.bg_vofs[0] = val & 0x1ff,
            REG_BG1HOFS => io.gpu.bg_hofs[1] = val & 0x1ff,
            REG_BG1VOFS => io.gpu.bg_vofs[1] = val & 0x1ff,
            REG_BG2HOFS => io.gpu.bg_hofs[2] = val & 0x1ff,
            REG_BG2VOFS => io.gpu.bg_vofs[2] = val & 0x1ff,
            REG_BG3HOFS => io.gpu.bg_hofs[3] = val & 0x1ff,
            REG_BG3VOFS => io.gpu.bg_vofs[3] = val & 0x1ff,
            REG_BG2X_L | REG_BG3X_L => write_reference_point!(low bg x internal_x),
            REG_BG2Y_L | REG_BG3Y_L => write_reference_point!(low bg y internal_y),
            REG_BG2X_H | REG_BG3X_H => write_reference_point!(high bg x internal_x),
            REG_BG2Y_H | REG_BG3Y_H => write_reference_point!(high bg y internal_y),
            REG_BG2PA => io.gpu.bg_aff[0].pa = val as i16,
            REG_BG2PB => io.gpu.bg_aff[0].pb = val as i16,
            REG_BG2PC => io.gpu.bg_aff[0].pc = val as i16,
            REG_BG2PD => io.gpu.bg_aff[0].pd = val as i16,
            REG_BG3PA => io.gpu.bg_aff[1].pa = val as i16,
            REG_BG3PB => io.gpu.bg_aff[1].pb = val as i16,
            REG_BG3PC => io.gpu.bg_aff[1].pc = val as i16,
            REG_BG3PD => io.gpu.bg_aff[1].pd = val as i16,
            REG_WIN0H => {
                let right = val & 0xff;
                let left = val >> 8;
                io.gpu.win0.right = right as u8;
                io.gpu.win0.left = left as u8;
            }
            REG_WIN1H => {
                let right = val & 0xff;
                let left = val >> 8;
                io.gpu.win1.right = right as u8;
                io.gpu.win1.left = left as u8;
            }
            REG_WIN0V => {
                let bottom = val & 0xff;
                let top = val >> 8;
                io.gpu.win0.bottom = bottom as u8;
                io.gpu.win0.top = top as u8;
            }
            REG_WIN1V => {
                let bottom = val & 0xff;
                let top = val >> 8;
                io.gpu.win1.bottom = bottom as u8;
                io.gpu.win1.top = top as u8;
            }
            REG_WININ => {
                let value = val & !0xc0c0;
                io.gpu.win0.flags = WindowFlags::from(value & 0xff);
                io.gpu.win1.flags = WindowFlags::from(value >> 8);
            }
            REG_WINOUT => {
                let value = val & !0xc0c0;
                io.gpu.winout_flags = WindowFlags::from(value & 0xff);
                io.gpu.winobj_flags = WindowFlags::from(value >> 8);
            }
            REG_MOSAIC => io.gpu.mosaic = val.into(),
            REG_BLDCNT => io.gpu.bldcnt.write(val),
            REG_BLDALPHA => io.gpu.bldalpha.write(val),
            REG_BLDY => io.gpu.bldy = cmp::min(val & 0b11111, 16),
            REG_IME => io.intc.master_enable = val != 0,
            REG_IE => io.intc.enable = val.into(),
            REG_IF => io.intc.clear(val),
            REG_TM0CNT_L..=REG_TM3CNT_H => println!("TODO TIMER"),
            SOUND_BASE..=SOUND_END => println!("TODO SOUND"),
            DMA_BASE..=REG_DMA3CNT_H => {
                let ofs = io_addr - DMA_BASE;
                let channel_id = (ofs / 12) as usize;
                io.dmac.write_16(channel_id, ofs % 12, val)
            }
            REG_WAITCNT => {
                io.waitcnt = val.into();
                (*io.sysbus_ptr).on_waitcnt_written(io.waitcnt);
            }
            REG_POSTFLG => io.post_boot_flag = val != 0,
            REG_HALTCNT => {
                if val & 0x80 != 0 {
                    io.haltcnt = HaltState::Stop;
                    panic!("Can't handle HaltCtrl == Stop yet");
                } else {
                    io.haltcnt = HaltState::Halt;
                }
            }
            _ => {
                let s = io_reg_string(io_addr);

                match s {
                    "UNKNOWN" => {
                        println!("Unimplemented write to 0x{:08X} {}", io_addr, s);
                    }
                    _ => {
                        panic!("Unimplemented write to 0x{:08X} {}", io_addr, s);
                    }
                }
            }
        }
    }

    fn read_32(&mut self, addr: Addr) -> u32 {
        self.read_16(addr) as u32 | (self.read_16(addr + 2) as u32) << 16
    }

    fn write_32(&mut self, addr: Addr, val: u32) {
        self.write_16(addr, (val & 0xffff) as u16);
        self.write_16(addr + 2, (val >> 16) as u16);
    }
}

const fn io_reg_string(addr: Addr) -> &'static str {
    match addr {
        REG_DISPCNT => "REG_DISPCNT",
        REG_DISPSTAT => "REG_DISPSTAT",
        REG_VCOUNT => "REG_VCOUNT",
        REG_BG0CNT => "REG_BG0CNT",
        REG_BG1CNT => "REG_BG1CNT",
        REG_BG2CNT => "REG_BG2CNT",
        REG_BG3CNT => "REG_BG3CNT",
        REG_BG0HOFS => "REG_BG0HOFS",
        REG_BG0VOFS => "REG_BG0VOFS",
        REG_BG1HOFS => "REG_BG1HOFS",
        REG_BG1VOFS => "REG_BG1VOFS",
        REG_BG2HOFS => "REG_BG2HOFS",
        REG_BG2VOFS => "REG_BG2VOFS",
        REG_BG3HOFS => "REG_BG3HOFS",
        REG_BG3VOFS => "REG_BG3VOFS",
        REG_BG2PA => "REG_BG2PA",
        REG_BG2PB => "REG_BG2PB",
        REG_BG2PC => "REG_BG2PC",
        REG_BG2PD => "REG_BG2PD",
        REG_BG2X_L => "REG_BG2X_L",
        REG_BG2X_H => "REG_BG2X_H",
        REG_BG2Y_L => "REG_BG2Y_L",
        REG_BG2Y_H => "REG_BG2Y_H",
        REG_BG3PA => "REG_BG3PA",
        REG_BG3PB => "REG_BG3PB",
        REG_BG3PC => "REG_BG3PC",
        REG_BG3PD => "REG_BG3PD",
        REG_BG3X_L => "REG_BG3X_L",
        REG_BG3X_H => "REG_BG3X_H",
        REG_BG3Y_L => "REG_BG3Y_L",
        REG_BG3Y_H => "REG_BG3Y_H",
        REG_WIN0H => "REG_WIN0H",
        REG_WIN1H => "REG_WIN1H",
        REG_WIN0V => "REG_WIN0V",
        REG_WIN1V => "REG_WIN1V",
        REG_WININ => "REG_WININ",
        REG_WINOUT => "REG_WINOUT",
        REG_MOSAIC => "REG_MOSAIC",
        REG_BLDCNT => "REG_BLDCNT",
        REG_BLDALPHA => "REG_BLDALPHA",
        REG_BLDY => "REG_BLDY",
        REG_SOUND1CNT_L => "REG_SOUND1CNT_L",
        REG_SOUND1CNT_H => "REG_SOUND1CNT_H",
        REG_SOUND1CNT_X => "REG_SOUND1CNT_X",
        REG_SOUND2CNT_L => "REG_SOUND2CNT_L",
        REG_SOUND2CNT_H => "REG_SOUND2CNT_H",
        REG_SOUND3CNT_L => "REG_SOUND3CNT_L",
        REG_SOUND3CNT_H => "REG_SOUND3CNT_H",
        REG_SOUND3CNT_X => "REG_SOUND3CNT_X",
        REG_SOUND4CNT_L => "REG_SOUND4CNT_L",
        REG_SOUND4CNT_H => "REG_SOUND4CNT_H",
        REG_SOUNDCNT_L => "REG_SOUNDCNT_L",
        REG_SOUNDCNT_H => "REG_SOUNDCNT_H",
        REG_SOUNDCNT_X => "REG_SOUNDCNT_X",
        REG_SOUNDBIAS => "REG_SOUNDBIAS",
        REG_WAVE_RAM => "REG_WAVE_RAM",
        REG_FIFO_A => "REG_FIFO_A",
        REG_FIFO_B => "REG_FIFO_B",
        REG_DMA0SAD => "REG_DMA0SAD",
        REG_DMA0DAD => "REG_DMA0DAD",
        REG_DMA0CNT_L => "REG_DMA0CNT_L",
        REG_DMA0CNT_H => "REG_DMA0CNT_H",
        REG_DMA1SAD => "REG_DMA1SAD",
        REG_DMA1DAD => "REG_DMA1DAD",
        REG_DMA1CNT_L => "REG_DMA1CNT_L",
        REG_DMA1CNT_H => "REG_DMA1CNT_H",
        REG_DMA2SAD => "REG_DMA2SAD",
        REG_DMA2DAD => "REG_DMA2DAD",
        REG_DMA2CNT_L => "REG_DMA2CNT_L",
        REG_DMA2CNT_H => "REG_DMA2CNT_H",
        REG_DMA3SAD => "REG_DMA3SAD",
        REG_DMA3DAD => "REG_DMA3DAD",
        REG_DMA3CNT_L => "REG_DMA3CNT_L",
        REG_DMA3CNT_H => "REG_DMA3CNT_H",
        REG_TM0CNT_L => "REG_TM0CNT_L",
        REG_TM0CNT_H => "REG_TM0CNT_H",
        REG_TM1CNT_L => "REG_TM1CNT_L",
        REG_TM1CNT_H => "REG_TM1CNT_H",
        REG_TM2CNT_L => "REG_TM2CNT_L",
        REG_TM2CNT_H => "REG_TM2CNT_H",
        REG_TM3CNT_L => "REG_TM3CNT_L",
        REG_TM3CNT_H => "REG_TM3CNT_H",
        // REG_SIODATA32 => "REG_SIODATA32",
        // REG_SIOMULTI0 => "REG_SIOMULTI0",
        // REG_SIOMULTI1 => "REG_SIOMULTI1",
        // REG_SIOMULTI2 => "REG_SIOMULTI2",
        // REG_SIOMULTI3 => "REG_SIOMULTI3",
        // REG_SIOCNT => "REG_SIOCNT",
        // REG_SIOMLT_SEND => "REG_SIOMLT_SEND",
        // REG_SIODATA8 => "REG_SIODATA8",
        REG_KEYINPUT => "REG_KEYINPUT",
        REG_KEYCNT => "REG_KEYCNT",
        REG_RCNT => "REG_RCNT",
        REG_IR => "REG_IR",
        REG_JOYCNT => "REG_JOYCNT",
        REG_JOY_RECV => "REG_JOY_RECV",
        REG_JOY_TRANS => "REG_JOY_TRANS",
        REG_JOYSTAT => "REG_JOYSTAT",
        REG_IE => "REG_IE",
        REG_IF => "REG_IF",
        REG_WAITCNT => "REG_WAITCNT",
        REG_IME => "REG_IME",
        REG_POSTFLG => "REG_POSTFLG",
        REG_HALTCNT => "REG_HALTCNT",
        REG_DEBUG_STRING => "REG_DEBUG_STRING",
        REG_DEBUG_FLAGS => "REG_DEBUG_FLAGS",
        REG_DEBUG_ENABLE => "REG_DEBUG_ENABLE",
        _ => "UNKNOWN",
    }
}

fn sign_extend_i32(value: i32, size: u32) -> i32 {
    let shift = 32 - size;
    ((value << shift) as i32) >> shift
}
