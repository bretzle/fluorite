use crate::{
    consts::*, dma::DmaController, gpu::Gpu, interrupt::InterruptController, sysbus::Bus,
    GpuMemoryMappedIO,
};
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
    pub fn new(gpu: Gpu, dmac: DmaController) -> Self {
        Self {
            gpu,
            intc: InterruptController::new(),
            dmac,
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

    fn write_8(&mut self, _addr: Addr, _val: u8) {
        todo!()
    }

    fn write_16(&mut self, addr: Addr, val: u16) {
        let io_addr = addr + IO_BASE;

        match io_addr {
            REG_DISPCNT => self.gpu.write_dispcnt(val),
            REG_DISPSTAT => self.gpu.dispstat.write(val),
            REG_HALTCNT => {
                if val & 0x80 != 0 {
                    self.haltcnt = HaltState::Stop;
                    panic!("Can't handle HaltCtrl == Stop yet");
                } else {
                    self.haltcnt = HaltState::Halt;
                }
            }
            DMA_BASE..=REG_DMA3CNT_H => {
                let ofs = io_addr - DMA_BASE;
                let channel_id = (ofs / 12) as usize;
                self.dmac.write_16(channel_id, ofs % 12, val)
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
