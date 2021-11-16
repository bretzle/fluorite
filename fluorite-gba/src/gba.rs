use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

use fluorite_arm::cpu::Arm7tdmi;
use fluorite_common::{Shared, WeakPointer};

use crate::consts::CYCLES_FULL_REFRESH;
use crate::dma::DmaController;
use crate::gpu::Gpu;
use crate::interrupt::IrqBitMask;
use crate::iodev::{HaltState, IoDevices};
use crate::sched::{EventType, Scheduler};
use crate::sysbus::SysBus;
use crate::VideoInterface;

pub const NUM_RENDER_TIMES: usize = 25;

pub struct Gba<T: VideoInterface> {
    cpu: Arm7tdmi<SysBus>,
    sysbus: Shared<SysBus>,
    io: Shared<IoDevices>,
    scheduler: Shared<Scheduler>,

    device: Rc<RefCell<T>>,
}

impl<T: VideoInterface> Gba<T> {
    pub fn new(device: Rc<RefCell<T>>, bios: &[u8], rom: &[u8]) -> Self {
        let interrupt_flags = Rc::new(Cell::new(IrqBitMask::new()));
        let scheduler = Shared::new(Scheduler::new());
        let gpu = Gpu::new(scheduler.clone(), interrupt_flags.clone());
        let dmac = DmaController::new(interrupt_flags, scheduler.clone());
        let mut io = Shared::new(IoDevices::new(gpu, dmac));
        let mut sysbus = Shared::new(SysBus::new(bios, rom, &scheduler, &io));
        let cpu = Arm7tdmi::new(sysbus.clone());

        io.set_sysbus_ptr(WeakPointer::new(&mut *sysbus as *mut SysBus));

        Self {
            cpu,
            sysbus,
            io,
            scheduler,
            device,
        }
    }

    pub fn skip_bios(&mut self) {
        self.cpu.skip_bios();
        self.io.gpu.skip_bios();
    }

    pub fn frame(&mut self) {
        // TODO: poll user input
        static mut EXTRA: usize = 0;
        unsafe {
            EXTRA = self.run(CYCLES_FULL_REFRESH - EXTRA);
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
            EventType::DmaActivateChannel(channel_id) => self.io.dmac.activate_channel(channel_id),
            EventType::TimerOverflow(_channel_id) => todo!(),
            EventType::Gpu(event) => {
                io.gpu
                    .on_event(event, cycles_late, &mut *self.sysbus, &self.device)
            }
            EventType::Apu(_event) => todo!(),
        }
    }

    fn dma_step(&mut self) {
        self.io.dmac.perform_work(&mut self.sysbus);
    }

    fn cpu_step(&mut self) {
        if self.io.intc.irq_pending() {
            todo!()
        }
        self.cpu.step();
    }

    pub fn render_time(&self) -> Duration {
        let sum: Duration = self.io.gpu.render_times.iter().sum();

        sum / (NUM_RENDER_TIMES as u32)
    }
}

#[derive(Debug, PartialEq)]
enum BusMaster {
    Dma,
    Cpu,
}
