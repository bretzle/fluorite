use std::any::Any;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use fluorite_arm::cpu::Arm7tdmi;
use fluorite_common::Shared;

use crate::consts::CYCLES_FULL_REFRESH;
use crate::iodev::{HaltState, IoDevices};
use crate::sched::{EventType, Scheduler};
use crate::sysbus::SysBus;

static BIOS: &[u8] = include_bytes!("../roms/gba_bios.bin");
static ROM: &[u8] = include_bytes!("../roms/beeg.bin");

pub struct Gba {
    cpu: Arm7tdmi<SysBus>,
    sysbus: Shared<SysBus>,
    io: Shared<IoDevices>,
    scheduler: Shared<Scheduler>,
}

impl Gba {
    pub fn new() -> Self {
        let io = Shared::new(IoDevices::new());
        let sysbus = Shared::new(SysBus::new(BIOS, ROM, &io));
        let cpu = Arm7tdmi::new(sysbus.clone());
        let scheduler = Shared::new(Scheduler::new());

        Self {
            cpu,
            sysbus,
            io,
            scheduler,
        }
    }

    pub fn skip_bios(&mut self) {
        self.cpu.skip_bios();
        // TODO: self.io.gpu.skip_bios();
    }

    pub fn frame(&mut self) {
        // TODO: poll user input
        static mut extra: usize = 0;
        unsafe {
            extra = self.run(CYCLES_FULL_REFRESH - extra);
        }
    }

    /// return number of extra cycles that ran
    fn run(&mut self, cycles_to_run: usize) -> usize {
        let run_start_time = self.scheduler.timestamp();

        // Register an event to mark the end of this run
        self.scheduler
            .push(EventType::RunLimitReached, cycles_to_run);

        let mut running = true;
        while running {
            // The tricky part is to avoid unnecessary calls for Scheduler::process_pending,
            // performance-wise it would be best to run as many cycles as fast as possible while we know there are no pending events.
            // Fast forward emulation until an event occurs
            while self.scheduler.timestamp() <= self.scheduler.timestamp_of_next_event() {
                // 3 Options:
                // 1. DMA is active - thus CPU is blocked
                // 2. DMA inactive and halt state is RUN - CPU can run
                // 3. DMA inactive and halt state is HALT - CPU is blocked
                match self.get_bus_master() {
                    Some(BusMaster::Dma) => self.dma_step(),
                    Some(BusMaster::Cpu) => self.cpu_step(),
                    None => {
                        if self.io.intc.irq_pending() {
                            self.io.haltcnt = HaltState::Running;
                        } else {
                            self.scheduler.fast_forward_to_next();
                            let (event, cycles_late) = self
                                .scheduler
                                .pop_pending_event()
                                .unwrap_or_else(|| unreachable!());
                            self.handle_event(event, cycles_late, &mut running);
                        }
                    }
                }
            }

            while let Some((event, cycles_late)) = self.scheduler.pop_pending_event() {
                self.handle_event(event, cycles_late, &mut running);
            }
        }

        let total_cycles_ran = self.scheduler.timestamp() - run_start_time;
        total_cycles_ran - cycles_to_run
    }

    fn get_bus_master(&mut self) -> Option<BusMaster> {
        match (self.io.dmac.is_active(), self.io.haltcnt) {
            (true, _) => Some(BusMaster::Dma),
            (false, HaltState::Running) => Some(BusMaster::Cpu),
            (false, _) => None,
        }
    }

    fn handle_event(&mut self, event: EventType, cycles_late: usize, running: &mut bool) {
        let io = &mut (*self.io);
        match event {
            EventType::RunLimitReached => {
                *running = false;
            }
            EventType::DmaActivateChannel(channel_id) => todo!(),
            EventType::TimerOverflow(channel_id) => todo!(),
            EventType::Gpu(event) => {
                io.gpu
                    .on_event(event, cycles_late, &mut *self.sysbus, &self.video_device)
            }
            EventType::Apu(event) => todo!(),
        }
    }
}

#[derive(Debug, PartialEq)]
enum BusMaster {
    Dma,
    Cpu,
}
