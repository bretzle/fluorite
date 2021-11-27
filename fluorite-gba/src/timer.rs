use fluorite_arm::Addr;
use fluorite_common::Shared;

use crate::consts::*;

use crate::dma::DmaController;
use crate::interrupt::signal_irq;
use crate::sched::EventType;
use crate::{
    interrupt::{Interrupt, SharedInterruptFlags},
    sched::Scheduler,
};

const SHIFT_LUT: [usize; 4] = [0, 6, 8, 10];

pub struct Timers {
    scheduler: Shared<Scheduler>,
    running_timers: u8,
    timers: [Timer; 4],
}

impl Timers {
    pub fn new(scheduler: Shared<Scheduler>, flags: SharedInterruptFlags) -> Self {
        Self {
            scheduler,
            running_timers: 0,
            timers: [
                Timer::new(0, flags.clone()),
                Timer::new(1, flags.clone()),
                Timer::new(2, flags.clone()),
                Timer::new(3, flags.clone()),
            ],
        }
    }

    pub fn handle_read(&mut self, addr: Addr) -> u16 {
        match addr {
            REG_TM0CNT_H => self.timers[0].ctl.0,
            REG_TM1CNT_H => self.timers[1].ctl.0,
            REG_TM2CNT_H => self.timers[2].ctl.0,
            REG_TM3CNT_H => self.timers[3].ctl.0,
            REG_TM0CNT_L => self.read_timer_data(0),
            REG_TM1CNT_L => self.read_timer_data(1),
            REG_TM2CNT_L => self.read_timer_data(2),
            REG_TM3CNT_L => self.read_timer_data(3),
            _ => unreachable!(),
        }
    }

    pub fn handle_write(&mut self, addr: Addr, val: u16) {
        match addr {
            REG_TM0CNT_L => {
                self.timers[0].data = val;
                self.timers[0].initial_data = val;
            }
            REG_TM0CNT_H => self.write_timer_ctl(0, val),

            REG_TM1CNT_L => {
                self.timers[1].data = val;
                self.timers[1].initial_data = val;
            }
            REG_TM1CNT_H => self.write_timer_ctl(1, val),

            REG_TM2CNT_L => {
                self.timers[2].data = val;
                self.timers[2].initial_data = val;
            }
            REG_TM2CNT_H => self.write_timer_ctl(2, val),

            REG_TM3CNT_L => {
                self.timers[3].data = val;
                self.timers[3].initial_data = val;
            }
            REG_TM3CNT_H => self.write_timer_ctl(3, val),
            _ => unreachable!(),
        }
    }

    fn write_timer_ctl(&mut self, id: usize, val: u16) {
        {
            let timer = &mut self.timers[id];
            let new_ctl = TimerCtl(val);
            let old_enabled = timer.ctl.enabled();
            let new_enabled = new_ctl.enabled();
            let cascade = new_ctl.cascade();
            timer.prescalar_shift = SHIFT_LUT[new_ctl.prescalar() as usize];
            timer.ctl = new_ctl;
            if new_enabled && !cascade {
                self.running_timers |= 1 << id;
                self.cancel_timer_event(id);
                self.add_timer_event(id, 0);
            } else {
                self.running_timers &= !(1 << id);
                self.cancel_timer_event(id);
            }
            // if old_enabled != new_enabled {
            //     println!(
            //         "TMR{} {}",
            //         id,
            //         if new_enabled { "enabled" } else { "disabled" }
            //     );
            // }
        }
        // println!("{:#?}", self.timers[id]);
    }

    fn add_timer_event(&mut self, id: usize, extra_cycles: usize) {
        let timer = &mut self.timers[id];
        timer.is_sceduled = true;
        timer.start_time = self.scheduler.timestamp() - extra_cycles;

        let cycles = (timer.ticks_to_overflow() as usize) << timer.prescalar_shift;
        self.scheduler
            .push(EventType::TimerOverflow(id), cycles - extra_cycles);
    }

    fn cancel_timer_event(&mut self, id: usize) {
        self.scheduler.cancel(EventType::TimerOverflow(id));
        self.timers[id].is_sceduled = false;
    }

    // TODO: when emu is paused this ticks the clock on a read
    fn read_timer_data(&mut self, id: usize) -> u16 {
        let timer = &mut self.timers[id];
        if timer.is_sceduled {
            // this timer is controlled by the scheduler so we need to manually calculate
            // the current value of the counter
            timer.sync_timer_data(self.scheduler.timestamp());
        }

        timer.data
    }

    pub fn handle_overflow_event(
        &mut self,
        id: usize,
        extra_cycles: usize,
        dmac: &mut DmaController,
    ) {
        self.handle_timer_overflow(id, dmac);
        self.add_timer_event(id, extra_cycles);
    }

    fn handle_timer_overflow(&mut self, id: usize, dmac: &mut DmaController) {
        self.timers[id].overflow();
        if id != 3 {
            let next_timer_id = id + 1;
            let next_timer = &mut self.timers[next_timer_id];
            if next_timer.ctl.cascade() {
                if next_timer.update(1) > 0 {
                    drop(next_timer);
                    self.handle_timer_overflow(next_timer_id, dmac);
                }
            }
        }
        // if id == 0 || id == 1 {
        //     apu.handle_timer_overflow(dmac, id, 1);
        // }
    }
}

#[derive(Clone, Debug)]
pub struct Timer {
    pub ctl: TimerCtl,
    pub data: u16,
    pub initial_data: u16,

    start_time: usize,
    is_sceduled: bool,

    irq: Interrupt,
    interrupt_flags: SharedInterruptFlags,
    timer_id: usize,
    prescalar_shift: usize,
}

impl Timer {
    pub fn new(timer_id: usize, flags: SharedInterruptFlags) -> Self {
        debug_assert!(timer_id <= 3);

        Self {
            ctl: TimerCtl::default(),
            data: 0,
            initial_data: 0,
            start_time: 0,
            is_sceduled: false,
            irq: Interrupt::from_usize(timer_id + 3).unwrap(),
            interrupt_flags: flags,
            timer_id,
            prescalar_shift: 0,
        }
    }

    fn ticks_to_overflow(&self) -> u32 {
        0x10000 - self.data as u32
    }

    fn overflow(&mut self) {
        self.data = self.initial_data;
        if self.ctl.irq_enabled() {
            signal_irq(&self.interrupt_flags, self.irq);
        }
    }

    fn update(&mut self, ticks: usize) -> usize {
        let mut ticks = ticks as u32;
        let mut num_overflows = 0;

        let ticks_remaining = self.ticks_to_overflow();

        if ticks >= ticks_remaining {
            num_overflows += 1;
            ticks -= ticks_remaining;
            self.data = self.initial_data;

            let ticks_remaining = self.ticks_to_overflow();
            num_overflows += ticks / ticks_remaining;
            ticks = ticks % ticks_remaining;

            if self.ctl.irq_enabled() {
                signal_irq(&self.interrupt_flags, self.irq);
            }
        }

        self.data += ticks as u16;

        num_overflows as usize
    }

    fn sync_timer_data(&mut self, timestamp: usize) {
        let ticks_passed = (timestamp - self.start_time) >> self.prescalar_shift;
        self.data += ticks_passed as u16;
    }
}

bitfield::bitfield! {
    #[derive(Clone, Default)]
    pub struct TimerCtl(u16);
    impl Debug;
    u16;
    prescalar, _ : 1, 0;
    cascade, _ : 2;
    irq_enabled, _ : 6;
    enabled, set_enabled : 7;
}
