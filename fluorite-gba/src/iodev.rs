use std::cmp;

use crate::{
    consts::*,
    dma::DmaController,
    gpu::{Gpu, WindowFlags},
    interrupt::InterruptController,
    keypad::KEYINPUT_ALL_RELEASED,
    sound::SoundController,
    sysbus::{Bus, SysBus},
    timer::Timers,
    GpuMemoryMappedIO,
};
use fluorite_arm::Addr;
use fluorite_common::WeakPointer;
use modular_bitfield::{bitfield, prelude::*};

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
    pub timers: Timers,
    pub keyinput: u16,
    pub sound: SoundController,
    rcnt: u16,

    sysbus_ptr: WeakPointer<SysBus>,
}

impl IoDevices {
    pub fn new(intc: InterruptController, gpu: Gpu, dmac: DmaController, timers: Timers, sound: SoundController) -> Self {
        Self {
            gpu,
            intc,
            dmac,
            haltcnt: HaltState::Running,
            waitcnt: WaitControl::new(),
            post_boot_flag: false,
            timers,
            keyinput: KEYINPUT_ALL_RELEASED,
            sound,
            rcnt: 0,
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
        let io_addr = addr | IO_BASE;

        match io_addr {
            REG_DISPCNT => self.gpu.dispcnt.read(),
            REG_DISPSTAT => self.gpu.dispstat.read(),
            REG_VCOUNT => self.gpu.vcount as u16,
            REG_BG0CNT => self.gpu.bgcnt[0].read(),
            REG_BG1CNT => self.gpu.bgcnt[1].read(),
            REG_BG2CNT => self.gpu.bgcnt[2].read(),
            REG_BG3CNT => self.gpu.bgcnt[3].read(),
            REG_WIN0H => ((self.gpu.win0.left as u16) << 8 | (self.gpu.win0.right as u16)),
            REG_WIN1H => ((self.gpu.win1.left as u16) << 8 | (self.gpu.win1.right as u16)),
            REG_WIN0V => ((self.gpu.win0.top as u16) << 8 | (self.gpu.win0.bottom as u16)),
            REG_WIN1V => ((self.gpu.win1.top as u16) << 8 | (self.gpu.win1.bottom as u16)),
            REG_WININ => {
                ((self.gpu.win1.flags.bits() as u16) << 8) | (self.gpu.win0.flags.bits() as u16)
            }
            REG_WINOUT => {
                ((self.gpu.winobj_flags.bits() as u16) << 8) | (self.gpu.winout_flags.bits() as u16)
            }
            REG_BLDCNT => self.gpu.bldcnt.read(),
            REG_BLDALPHA => self.gpu.bldalpha.read(),

            REG_IME => self.intc.master_enable as u16,
            REG_IE => self.intc.enable.into(),
            REG_IF => self.intc.flags.get().into(),

            REG_TM0CNT_L..=REG_TM3CNT_H => self.timers.handle_read(io_addr),

            SOUND_BASE..=SOUND_END => self.sound.handle_read(io_addr),
            REG_DMA0CNT_H => self.dmac.channels[0].ctrl.0,
            REG_DMA1CNT_H => self.dmac.channels[1].ctrl.0,
            REG_DMA2CNT_H => self.dmac.channels[2].ctrl.0,
            REG_DMA3CNT_H => self.dmac.channels[3].ctrl.0,
            // Even though these registers are write only,
            // some games may still try to read them.
            // TODO: should this be treated as an open-bus read?
            REG_DMA0CNT_L => 0,
            REG_DMA1CNT_L => 0,
            REG_DMA2CNT_L => 0,
            REG_DMA3CNT_L => 0,

            REG_WAITCNT => self.waitcnt.into(),

            REG_POSTFLG => self.post_boot_flag as u16,
            REG_HALTCNT => 0,
            REG_KEYINPUT => self.keyinput as u16,
            REG_KEYCNT => todo!(), // TODO
            REG_JOYCNT => 0, // TODO

            0x04000400..=0x04FFFFFF
            | 0x05000400..=0x05FFFFFF
            | 0x06018000..=0x06FFFFFF
            | 0x07000400..=0x07FFFFFF => 0, // Not used

            _ => {
                let s = io_reg_string(io_addr);

                match s {
                    "UNKNOWN" => {
                        println!("Unimplemented 16read from 0x{:08X} {}", io_addr, s);
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
            REG_TM0CNT_L..=REG_TM3CNT_H => io.timers.handle_write(io_addr, val),
            SOUND_BASE..=SOUND_END => io.sound.handle_write(io_addr, val),
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
            REG_KEYCNT => println!("WRITE TO REG_KEYCNT (0x{:X})", val),
            REG_KEYINPUT => io.keyinput = val,
            REG_RCNT => io.rcnt = val,

            REG_JOYCNT => println!("WRITE TO REG_JOYCNT (0x{:X})", val),
            REG_JOY_RECV => println!("WRITE TO REG_JOY_RECV (0x{:X})", val),
            REG_JOY_TRANS => println!("WRITE TO REG_JOY_TRANS (0x{:X})", val),
            REG_JOYSTAT => println!("WRITE TO REG_JOYSTAT (0x{:X})", val),

            _ => {
                let s = io_reg_string(io_addr);

                match s {
                    "UNKNOWN" => {
                        // println!("Unimplemented write to 0x{:08X} {}", io_addr, s);
                    }
                    _ => {
                        panic!("Unimplemented write to 0x{:08X} {}", io_addr, s);
                    }
                }
            }
        }
    }
}

fn sign_extend_i32(value: i32, size: u32) -> i32 {
    let shift = 32 - size;
    ((value << shift) as i32) >> shift
}
