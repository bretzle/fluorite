use fluorite_arm::memory::{MemoryAccess, MemoryInterface};
use fluorite_common::Shared;

use crate::{
    consts::{REG_FIFO_A, REG_FIFO_B},
    interrupt::{self, Interrupt, SharedInterruptFlags},
    sched::{EventType, Scheduler},
    sysbus::SysBus,
};

pub const TIMING_VBLANK: u16 = 1;
pub const TIMING_HBLANK: u16 = 2;

pub trait DmaNotifier {
    fn notify(&mut self, timing: u16);
}

#[derive(Clone, Debug)]
pub struct DmaController {
    pub channels: [DmaChannel; 4],
    pending_set: u8,
    scheduler: Shared<Scheduler>,
}

impl DmaController {
    pub fn new(interrupt_flags: SharedInterruptFlags, scheduler: Shared<Scheduler>) -> Self {
        Self {
            channels: [
                DmaChannel::new(0, interrupt_flags.clone()),
                DmaChannel::new(1, interrupt_flags.clone()),
                DmaChannel::new(2, interrupt_flags.clone()),
                DmaChannel::new(3, interrupt_flags),
            ],
            pending_set: 0,
            scheduler,
        }
    }

    pub fn is_active(&self) -> bool {
        self.pending_set != 0
    }

    pub fn perform_work(&mut self, bus: &mut SysBus) {
        for id in 0..4 {
            if self.pending_set & (1 << id) != 0 {
                self.channels[id].transfer(bus);
            }
        }
        self.pending_set = 0;
    }

    pub fn write_16(&mut self, channel_id: usize, ofs: u32, val: u16) {
        match ofs {
            0 => self.channels[channel_id].write_src_low(val),
            2 => self.channels[channel_id].write_src_high(val),
            4 => self.channels[channel_id].write_dst_low(val),
            6 => self.channels[channel_id].write_dst_high(val),
            8 => self.channels[channel_id].write_word_count(val),
            10 => {
                let start_immediately = self.channels[channel_id].write_dma_ctrl(val);
                if start_immediately {
                    // DMA actually starts after 3 cycles
                    self.scheduler
                        .push(EventType::DmaActivateChannel(channel_id), 3);
                } else {
                    self.deactivate_channel(channel_id);
                }
            }
            _ => panic!("Invalid dma offset 0x{:X}", ofs),
        }
    }

    pub fn notify_from_gpu(&mut self, timing: u16) {
        for i in 0..4 {
            if self.channels[i].ctrl.is_enabled() && self.channels[i].ctrl.timing() == timing {
                self.pending_set |= 1 << i;
            }
        } 
    }

    pub fn activate_channel(&mut self, channel_id: usize) {
        self.pending_set |= 1 << channel_id;
    }

    pub fn deactivate_channel(&mut self, channel_id: usize) {
        self.pending_set &= !(1 << channel_id);
    }
}

#[derive(Clone, Debug, Default)]
struct DmaInternalRegs {
    src_addr: u32,
    dst_addr: u32,
    count: u32,
}

bitfield::bitfield! {
    #[derive(Clone, Default)]
    pub struct DmaChannelCtrl(u16);
    impl Debug;
    u16;
    dst_adj, _ : 6, 5;
    src_adj, _ : 8, 7;
    repeat, _ : 9;
    is_32bit, _: 10;
    timing, _: 13, 12;
    is_triggering_irq, _: 14;
    is_enabled, set_enabled: 15;
}

#[derive(Clone, Debug)]
pub struct DmaChannel {
    id: usize,

    pub src: u32,
    pub dst: u32,
    pub wc: u32,
    pub ctrl: DmaChannelCtrl,

    // These are "latched" when the dma is enabled.
    internal: DmaInternalRegs,

    running: bool,
    fifo_mode: bool,
    irq: Interrupt,
    interrupt_flags: SharedInterruptFlags,
}

impl DmaChannel {
    fn new(id: usize, interrupt_flags: SharedInterruptFlags) -> DmaChannel {
        if id > 3 {
            panic!("invalid dma id {}", id);
        }
        DmaChannel {
            id,
            irq: Interrupt::from_usize(id + 8).unwrap(),
            running: false,
            src: 0,
            dst: 0,
            wc: 0,
            ctrl: DmaChannelCtrl(0),

            fifo_mode: false,
            internal: Default::default(),
            interrupt_flags,
        }
    }

    fn write_src_low(&mut self, low: u16) {
        let src = self.src;
        self.src = (src & 0xffff0000) | (low as u32);
    }

    fn write_src_high(&mut self, high: u16) {
        let src = self.src;
        let high = (high & 0xfff) as u32;
        self.src = (src & 0xffff) | (high << 16);
    }

    fn write_dst_low(&mut self, low: u16) {
        let dst = self.dst;
        self.dst = (dst & 0xffff0000) | (low as u32);
    }

    fn write_dst_high(&mut self, high: u16) {
        let dst = self.dst;
        let high = (high & 0xfff) as u32;
        self.dst = (dst & 0xffff) | (high << 16);
    }

    fn write_word_count(&mut self, value: u16) {
        self.wc = value as u32;
    }

    pub fn write_dma_ctrl(&mut self, value: u16) -> bool {
        let ctrl = DmaChannelCtrl(value);
        let timing = ctrl.timing();
        let mut start_immediately = false;
        if ctrl.is_enabled() && !self.ctrl.is_enabled() {
            println!(
                "DMA{} enabled! timing={} src={:#x} dst={:#x} cnt={}",
                self.id, timing, self.src, self.dst, self.wc
            );
            self.running = true;
            start_immediately = timing == 0;
            self.internal.src_addr = self.src;
            self.internal.dst_addr = self.dst;
            self.internal.count = self.wc;
            self.fifo_mode = timing == 3
                && ctrl.repeat()
                && (self.id == 1 || self.id == 2)
                && (self.dst == REG_FIFO_A || self.dst == REG_FIFO_B);
        }
        if !ctrl.is_enabled() {
            self.running = false;
        }
        self.ctrl = ctrl;

        start_immediately
    }

    fn transfer(&mut self, bus: &mut SysBus) {
        let word_size = if self.ctrl.is_32bit() { 4 } else { 2 };
        let count = match self.internal.count {
            0 => match self.id {
                3 => 0x1_0000,
                _ => 0x0_4000,
            },
            _ => self.internal.count,
        };

        if self.id == 3 && word_size == 2 {
            // TODO:
            // if let BackupMedia::Eeprom(eeprom) = &mut bus.cartridge.backup {
            //     eeprom.on_dma3_transfer(
            //         self.internal.src_addr,
            //         self.internal.dst_addr,
            //         count as usize,
            //     )
            // }
        }

        let fifo_mode = self.fifo_mode;

        let src_adj = match self.ctrl.src_adj() {
            /* Increment */ 0 => word_size,
            /* Decrement */ 1 => 0 - word_size,
            /* Fixed */ 2 => 0,
            _ => panic!("forbidden DMA source address adjustment"),
        };
        let dst_adj = match self.ctrl.dst_adj() {
            /* Increment[+Reload] */ 0 | 3 => word_size,
            /* Decrement */ 1 => 0 - word_size,
            /* Fixed */ 2 => 0,
            _ => panic!("forbidden DMA dest address adjustment"),
        };

        let mut access = MemoryAccess::NonSeq;
        if fifo_mode {
            for _ in 0..4 {
                let v = bus.load_32(self.internal.src_addr & !3, access);
                bus.store_32(self.internal.dst_addr & !3, v, access);
                access = MemoryAccess::Seq;
                self.internal.src_addr += 4;
            }
        } else if word_size == 4 {
            for _ in 0..count {
                let w = bus.load_32(self.internal.src_addr & !3, access);
                bus.store_32(self.internal.dst_addr & !3, w, access);
                access = MemoryAccess::Seq;
                self.internal.src_addr += src_adj;
                self.internal.dst_addr += dst_adj;
            }
        } else {
            for _ in 0..count {
                let hw = bus.load_16(self.internal.src_addr & !1, access);
                bus.store_16(self.internal.dst_addr & !1, hw, access);
                access = MemoryAccess::Seq;
                self.internal.src_addr += src_adj;
                self.internal.dst_addr += dst_adj;
            }
        }
        if self.ctrl.is_triggering_irq() {
            interrupt::signal_irq(&self.interrupt_flags, self.irq);
        }
        if self.ctrl.repeat() {
            /* reload */
            if 3 == self.ctrl.dst_adj() {
                self.internal.dst_addr = self.dst;
            }
        } else {
            self.running = false;
            self.ctrl.set_enabled(false);
        }
    }
}
