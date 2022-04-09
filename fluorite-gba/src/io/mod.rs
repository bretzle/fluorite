use self::{
    apu::Apu,
    dma::Dma,
    gamepak::{
        gpio::{Gpio, GpioDevice},
        Gamepak,
    },
    gpu::Gpu,
    interrupt_controller::InterruptController,
    keypad::{Keypad, KEYINPUT},
    memory::{MemoryRegion, MemoryValue},
    scheduler::{Event, EventType, Scheduler},
    timers::Timers,
};
use crate::{consts::CLOCK_FREQ, io::interrupt_controller::InterruptRequest, BIOS};
use fluorite_common::flume::Receiver;
use num::FromPrimitive;
use std::{cell::Cell, collections::VecDeque, mem::size_of};

pub mod apu;
pub mod dma;
pub mod gamepak;
pub mod gpu;
pub mod interrupt_controller;
pub mod keypad;
pub mod memory;
pub mod scheduler;
pub mod timers;

#[derive(Clone, Copy)]
pub enum MemoryAccess {
    N,
    S,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Cycle {
    N,
    S,
    I,
}

impl From<MemoryAccess> for Cycle {
    fn from(val: MemoryAccess) -> Self {
        match val {
            MemoryAccess::N => Cycle::N,
            MemoryAccess::S => Cycle::S,
        }
    }
}

pub struct Sysbus {
    // pub rom: Box<[u8]>,
    pub gamepak: Gamepak,

    ewram: Box<[u8]>,
    iwram: Box<[u8]>,

    scheduler: Scheduler,
    clocks_ahead: u32,

    // Devices
    pub gpu: Gpu,
    apu: Apu,
    dma: Dma,
    timers: Timers,
    keypad: Keypad,
    interrupt_controller: InterruptController,
    _rtc: (),
    _backup: (),

    // registers
    haltcnt: u16,
    waitcnt: WaitStateControl,

    // open bus
    pc: u32,
    in_thumb: bool,
    pipeline: [u32; 2],
    bios_latch: Cell<u32>,

    mgba_test_suite: mgba_test_suite::MGBATestSuite,
}

impl Sysbus {
    const EWRAM_MASK: u32 = 0x3FFFF;
    const IWRAM_MASK: u32 = 0x7FFF;

    pub fn new(rx: Receiver<(KEYINPUT, bool)>) -> Self {
        Self {
            gamepak: Gamepak::new(),

            ewram: vec![0; 0x40000].into_boxed_slice(),
            iwram: vec![0; 0x8000].into_boxed_slice(),

            scheduler: Scheduler::new(),
            clocks_ahead: 0,

            gpu: Gpu::new(),
            apu: Apu::new(),
            dma: Dma::new(),
            timers: Timers::new(),
            keypad: Keypad::new(rx),
            interrupt_controller: InterruptController::new(),
            _rtc: (),
            _backup: (),

            haltcnt: 0,
            waitcnt: WaitStateControl::new(),

            pc: 0,
            in_thumb: false,
            pipeline: [0; 2],
            bios_latch: Cell::new(0xE129F000),

            mgba_test_suite: mgba_test_suite::MGBATestSuite::new(),
        }
    }

    pub fn reset(&mut self) {
        self.ewram.fill(0);
        self.iwram.fill(0);
        self.scheduler = Scheduler::new();
        self.clocks_ahead = 0;
        self.gpu = Gpu::new();
        self.apu = Apu::new();
        self.dma = Dma::new();
        self.timers = Timers::new();
        self.keypad.reset();
        self.interrupt_controller = InterruptController::new();
        self._rtc = ();
        self._backup = ();
        self.haltcnt = 0;
        self.waitcnt = WaitStateControl::new();
        self.pc = 0;
        self.in_thumb = false;
        self.pipeline = [0; 2];
        self.bios_latch.set(0xE129F000);
    }

    pub fn read<T>(&self, addr: u32) -> T
    where
        T: MemoryValue,
    {
        match MemoryRegion::get_region(addr) {
            MemoryRegion::Bios => self.read_bios(addr),
            MemoryRegion::Ewram => Self::read_mem(&self.ewram, addr & Self::EWRAM_MASK),
            MemoryRegion::Iwram => Self::read_mem(&self.iwram, addr & Self::IWRAM_MASK),
            MemoryRegion::Io => Self::read_from_bytes(self, &Self::read_io_register, addr),
            MemoryRegion::Palette => Self::read_from_bytes(&self.gpu, &Gpu::read_palette_ram, addr),
            MemoryRegion::Vram => Self::read_mem(&self.gpu.vram, Gpu::parse_vram_addr(addr)),
            MemoryRegion::Oam => Self::read_mem(&self.gpu.oam, Gpu::parse_oam_addr(addr)),
            MemoryRegion::Rom0L => {
                if (0x080000C4..=0x80000C9).contains(&addr)
                    && self.gamepak.is_rtc_used()
                    && !self.gamepak.gpio.write_only()
                {
                    Self::read_from_bytes(
                        &self.gamepak.gpio,
                        &Gpio::read_register,
                        addr - 0x080000C4,
                    )
                } else {
                    self.read_rom(addr)
                }
            }
            MemoryRegion::Rom0H => self.read_rom(addr),
            MemoryRegion::Rom1L => self.read_rom(addr),
            MemoryRegion::Rom1H => self.read_rom(addr),
            MemoryRegion::Rom2L => self.read_rom(addr),
            MemoryRegion::Rom2H => todo!(),
            MemoryRegion::Sram => self.read_sram(addr),
            MemoryRegion::Unused => self.read_openbus(addr),
        }
    }

    pub fn write<T>(&mut self, addr: u32, value: T)
    where
        T: MemoryValue,
    {
        match MemoryRegion::get_region(addr) {
            MemoryRegion::Bios => (),
            MemoryRegion::Ewram => Self::write_mem(&mut self.ewram, addr & Self::EWRAM_MASK, value),
            MemoryRegion::Iwram => Self::write_mem(&mut self.iwram, addr & Self::IWRAM_MASK, value),
            MemoryRegion::Io => Self::write_from_bytes(self, &Self::write_register, addr, value),
            MemoryRegion::Palette => self.write_palette_ram(addr, value),
            MemoryRegion::Vram => self.write_vram(Gpu::parse_vram_addr(addr), value),
            MemoryRegion::Oam => self.write_oam(Gpu::parse_oam_addr(addr), value),
            MemoryRegion::Rom0L => {
                if (0x080000C4..=0x80000C9).contains(&addr) && self.gamepak.is_rtc_used() {
                    Self::write_from_bytes(
                        &mut self.gamepak.gpio,
                        &Gpio::write_register,
                        addr - 0x080000C4,
                        value,
                    )
                }
            }
            MemoryRegion::Rom0H => todo!(),
            MemoryRegion::Rom1L => todo!(),
            MemoryRegion::Rom1H => todo!(),
            MemoryRegion::Rom2L => todo!(),
            MemoryRegion::Rom2H => todo!(),
            MemoryRegion::Sram => self.write_sram(addr, value),
            MemoryRegion::Unused => {}
        }
    }

    pub fn inc_clock<C: Into<Cycle>>(&mut self, cycle: C, addr: u32, access_width: u32) {
        let cycle = cycle.into();
        let clocks_inc = if cycle == Cycle::I {
            1
        } else {
            match MemoryRegion::get_region(addr) {
                MemoryRegion::Bios => 1,                                 // BIOS ROM
                MemoryRegion::Ewram => [3, 3, 6][access_width as usize], // WRAM - On-board 256K
                MemoryRegion::Iwram => 1,
                MemoryRegion::Io => 1,
                MemoryRegion::Palette => {
                    if access_width < 2 {
                        1
                    } else {
                        2
                    }
                }
                MemoryRegion::Vram => {
                    if access_width < 2 {
                        1
                    } else {
                        2
                    }
                }
                MemoryRegion::Oam => 1,
                MemoryRegion::Rom0L | MemoryRegion::Rom0H => {
                    self.waitcnt
                        .get_rom_access_time(0, cycle, access_width, addr)
                }
                MemoryRegion::Rom1L | MemoryRegion::Rom1H => {
                    self.waitcnt
                        .get_rom_access_time(1, cycle, access_width, addr)
                }
                MemoryRegion::Rom2L | MemoryRegion::Rom2H => {
                    self.waitcnt
                        .get_rom_access_time(2, cycle, access_width, addr)
                }
                MemoryRegion::Sram => self.waitcnt.get_sram_access_time(cycle),
                MemoryRegion::Unused => 1,
            }
        };
        self.waitcnt.clock_prefetch(clocks_inc);

        for _ in 0..clocks_inc {
            self.handle_events();
            // self.gamepak.gpio.clock();
            self.apu.clock();
        }
        self.clocks_ahead += clocks_inc;
        while self.clocks_ahead >= 4 {
            self.clocks_ahead -= 4;
            self.interrupt_controller.request |= self.gpu.emulate_dot();
        }
    }

    pub fn handle_events(&mut self) {
        self.scheduler.cycle += 1;
        while let Some(event) = self.scheduler.get_next_event() {
            self.handle_event(event);
        }
    }

    pub fn handle_event(&mut self, event: EventType) {
        match event {
            EventType::TimerOverflow(timer) => {
                if self.timers.timers[timer].cnt.irq {
                    self.interrupt_controller.request |= self.timers.timers[timer].interrupt
                }
                // Cascade Timers
                if timer + 1 < self.timers.timers.len()
                    && self.timers.timers[timer + 1].is_count_up()
                    && self.timers.timers[timer + 1].clock()
                {
                    self.handle_event(EventType::TimerOverflow(timer + 1))
                }
                if !self.timers.timers[timer].is_count_up() {
                    self.timers.timers[timer].reload();
                    self.timers.timers[timer].create_event(&mut self.scheduler, 0);
                }
                self.apu.on_timer_overflowed(timer);
            }
            EventType::FrameSequencer(step) => {
                self.apu.clock_sequencer(step);
                self.scheduler.add(Event {
                    cycle: self.scheduler.cycle + (CLOCK_FREQ / 512),
                    event_type: EventType::FrameSequencer((step + 1) % 8),
                });
            }
        }
    }

    pub fn interrupts_requested(&mut self) -> bool {
        // if self.keypad.interrupt_requested() {
        //     self.interrupt_controller.request |= InterruptRequest::KEYPAD
        // }

        self.interrupt_controller.master_enable.bits() != 0
            && (self.interrupt_controller.request.bits() & self.interrupt_controller.enable.bits())
                != 0
    }

    pub fn get_cycle(&self) -> usize {
        self.scheduler.cycle
    }

    pub fn poll_keypad_updates(&mut self) {
        if self.gpu.rendered_frame() {
            // TODO: write save to file
            self.keypad.poll();
        }
    }

    pub fn run_dma(&mut self) {
        let dma_channel = self.dma.get_channel_running(
            self.gpu.hblank_called(),
            self.gpu.vblank_called(),
            [self.apu.fifo_a_req(), self.apu.fifo_b_req()],
        );
        if dma_channel < 4 {
            self.dma.in_dma = true;
            let channel = &mut self.dma.channels[dma_channel];
            let is_fifo = (channel.num == 1 || channel.num == 2) && channel.cnt.start_timing == 3;
            let count = if is_fifo { 4 } else { channel.count_latch };
            let mut src_addr = channel.sad_latch;
            let mut dest_addr = channel.dad_latch;
            let src_addr_ctrl = channel.cnt.src_addr_ctrl;
            let dest_addr_ctrl = if is_fifo {
                2
            } else {
                channel.cnt.dest_addr_ctrl
            };
            let transfer_32 = if is_fifo {
                true
            } else {
                channel.cnt.transfer_32
            };
            let irq = channel.cnt.irq;
            channel.cnt.enable = channel.cnt.start_timing != 0 && channel.cnt.repeat;
            // println!(
            //     "Running DMA{}: Writing {} values to {:08X} from {:08X}, size: {}",
            //     dma_channel,
            //     count,
            //     dest_addr,
            //     src_addr,
            //     if transfer_32 { 32 } else { 16 }
            // );

            // TODO:
            // if MemoryRegion::get_region(dest_addr) == MemoryRegion::ROM2H
            //     && self.cart_backup.is_eeprom_access(dest_addr, self.rom.len())
            // {
            //     self.cart_backup.init_eeprom(count)
            // }

            let (access_width, addr_change, addr_mask) = if transfer_32 {
                (2, 4, 0x3)
            } else {
                (1, 2, 0x1)
            };
            src_addr &= !addr_mask;
            dest_addr &= !addr_mask;
            let mut first = true;
            let original_dest_addr = dest_addr;
            for _ in 0..count {
                let cycle_type = if first { Cycle::N } else { Cycle::S };
                self.inc_clock(cycle_type, src_addr, access_width);
                self.inc_clock(cycle_type, dest_addr, access_width);
                if transfer_32 {
                    self.write::<u32>(dest_addr, self.read::<u32>(src_addr))
                } else {
                    self.write::<u16>(dest_addr, self.read::<u16>(src_addr))
                }

                src_addr = match src_addr_ctrl {
                    0 => src_addr.wrapping_add(addr_change),
                    1 => src_addr.wrapping_sub(addr_change),
                    2 => src_addr,
                    _ => panic!("Invalid DMA Source Address Control!"),
                };
                dest_addr = match dest_addr_ctrl {
                    0 | 3 => dest_addr.wrapping_add(addr_change),
                    1 => dest_addr.wrapping_sub(addr_change),
                    2 => dest_addr,
                    _ => unreachable!(),
                };
                first = false;
            }
            let channel = &mut self.dma.channels[dma_channel];
            channel.sad_latch = src_addr;
            channel.dad_latch = dest_addr;
            if channel.cnt.enable {
                channel.count_latch = channel.count.count as u32
            } // Only reload Count
            if dest_addr_ctrl == 3 {
                channel.dad_latch = original_dest_addr
            }
            for _ in 0..2 {
                self.inc_clock(Cycle::I, 0, 0)
            }

            if irq {
                self.interrupt_controller.request |= match dma_channel {
                    0 => InterruptRequest::DMA0,
                    1 => InterruptRequest::DMA1,
                    2 => InterruptRequest::DMA2,
                    3 => InterruptRequest::DMA3,
                    _ => unreachable!(),
                }
            }
            self.dma.in_dma = false;
        }
    }

    fn read_mem<T>(mem: &[u8], addr: u32) -> T
    where
        T: MemoryValue,
    {
        unsafe { *(&mem[addr as usize] as *const u8 as *const T) }
    }

    fn write_mem<T>(mem: &mut [u8], addr: u32, value: T)
    where
        T: MemoryValue,
    {
        unsafe {
            *(&mut mem[addr as usize] as *mut u8 as *mut T) = value;
        }
    }
}

impl Sysbus {
    fn read_from_bytes<T, F, D>(device: &D, read_fn: &F, addr: u32) -> T
    where
        T: MemoryValue,
        F: Fn(&D, u32) -> u8,
    {
        let mut value: T = num::zero();
        for i in 0..size_of::<T>() as u32 {
            value =
                num::cast::<u8, T>(read_fn(device, addr + i)).unwrap() << (8 * i as usize) | value;
        }
        value
    }

    fn read_bios<T>(&self, addr: u32) -> T
    where
        T: MemoryValue,
    {
        if self.pc < 0x4000 {
            self.bios_latch.set(Self::read_mem(BIOS, addr)); // Always 32 bit read
            Self::read_mem(BIOS, addr)
        } else {
            let mask = match size_of::<T>() {
                1 => 0xFF,
                2 => 0xFFFF,
                4 => 0xFFFF_FFFF,
                _ => unreachable!(),
            };
            FromPrimitive::from_u32((self.bios_latch.get() >> ((addr & 3) * 8)) & mask).unwrap()
        }
    }

    fn read_rom<T>(&self, addr: u32) -> T
    where
        T: MemoryValue,
    {
        let addr = addr - 0x08000000;
        if (addr as usize) < self.gamepak.rom.len() {
            Self::read_mem(self.gamepak.rom.as_ref(), addr)
        } else {
            num::zero()
        }
    }

    fn read_sram<T>(&self, addr: u32) -> T
    where
        T: MemoryValue,
    {
        if self.gamepak.is_eeprom() {
            return match size_of::<T>() {
                1 => FromPrimitive::from_u8(0xFF).unwrap(),
                2 => FromPrimitive::from_u16(0xFFFF).unwrap(),
                4 => FromPrimitive::from_u32(0xFFFF_FFFF).unwrap(),
                _ => unreachable!(),
            };
        }
        let addr = addr & 0x0EFFFFFF;
        let byte = FromPrimitive::from_u8(self.read_cart_backup(addr - 0x0E000000)).unwrap();
        match size_of::<T>() {
            1 => byte,
            2 => byte * FromPrimitive::from_u16(0x0101).unwrap(),
            4 => byte * FromPrimitive::from_u32(0x01010101).unwrap(),
            _ => unreachable!(),
        }
    }

    fn read_cart_backup(&self, addr: u32) -> u8 {
        self.gamepak.read_save(addr)
    }

    fn read_io_register(&self, addr: u32) -> u8 {
        match addr {
            0x04000000..=0x0400005F => self.gpu.read_register(addr),
            0x04000060..=0x040000AF => self.apu.read_register(addr),
            0x040000B0..=0x040000BB => self.dma.channels[0].read(addr as u8 - 0xB0),
            0x040000BC..=0x040000C7 => self.dma.channels[1].read(addr as u8 - 0xBC),
            0x040000C8..=0x040000D3 => self.dma.channels[2].read(addr as u8 - 0xC8),
            0x040000D4..=0x040000DF => self.dma.channels[3].read(addr as u8 - 0xD4),
            0x040000E0..=0x040000FF => 0,
            0x04000100..=0x04000103 => self.timers.timers[0].read(&self.scheduler, addr as u8 % 4),
            0x04000104..=0x04000107 => self.timers.timers[1].read(&self.scheduler, addr as u8 % 4),
            0x04000108..=0x0400010B => self.timers.timers[2].read(&self.scheduler, addr as u8 % 4),
            0x0400010C..=0x0400010F => self.timers.timers[3].read(&self.scheduler, addr as u8 % 4),
            0x04000120..=0x0400012F => {
                warn!("Read from SerialCom(1)");
                0
            }
            0x04000130 => self.keypad.keyinput.read::<0>(),
            0x04000131 => self.keypad.keyinput.read::<1>(),
            0x04000132 => self.keypad.keycnt.read::<0>(),
            0x04000133 => self.keypad.keycnt.read::<1>(),
            0x04000134..=0x04000159 => todo!(),
            0x0400015A..=0x040001FF => 0,
            0x04000200 => self.interrupt_controller.enable.read::<0>(),
            0x04000201 => self.interrupt_controller.enable.read::<1>(),
            0x04000202 => self.interrupt_controller.request.read::<0>(),
            0x04000203 => self.interrupt_controller.request.read::<1>(),
            0x04000204 => self.waitcnt.read(0),
            0x04000205 => self.waitcnt.read(1),
            0x04000206..=0x04000207 => 0, // Unused IO Register
            0x04000208 => self.interrupt_controller.master_enable.read::<0>(),
            0x04000209 => self.interrupt_controller.master_enable.read::<1>(),
            0x0400020A..=0x040002FF => 0, // Unused IO Register
            0x04000300 => self.haltcnt as u8,
            0x04000301 => (self.haltcnt >> 8) as u8,
            0x04FFF780..=0x04FFF781 => self.mgba_test_suite.read_register(addr),
            // 0x04000000..=0x04700000 => panic!("Reading Unimplemented IO Register at {addr:08X}"),
            _ => 0,
        }
    }

    fn read_openbus<T>(&self, addr: u32) -> T
    where
        T: MemoryValue,
    {
        use MemoryRegion::*;
        let value = if self.in_thumb {
            match MemoryRegion::get_region(self.pc) {
                Ewram | Palette | Vram | Rom0L | Rom0H | Rom1L | Rom1H | Rom2L | Rom2H => {
                    self.pipeline[1] * 0x00010001
                }
                Bios | Oam => self.pipeline[0] | self.pipeline[1] << 16,
                Iwram if self.pc & 0x3 != 0 => self.pipeline[0] | self.pipeline[1] << 16,
                Iwram => self.pipeline[1] | self.pipeline[0] << 16,
                Io | Sram | Unused => 0,
            }
        } else {
            self.pipeline[1]
        };
        let mask = match std::mem::size_of::<T>() {
            1 => 0xFF,
            2 => 0xFFFF,
            4 => 0xFFFF_FFFF,
            _ => unreachable!(),
        };
        FromPrimitive::from_u32((value >> ((addr & 3) * 8)) & mask).unwrap()
    }
}

impl Sysbus {
    fn write_from_bytes<T, F, D>(device: &mut D, write_fn: &F, addr: u32, value: T)
    where
        T: MemoryValue,
        F: Fn(&mut D, u32, u8),
    {
        let mask = FromPrimitive::from_u8(0xFF).unwrap();
        for i in 0..size_of::<T>() {
            write_fn(
                device,
                addr + i as u32,
                num::cast::<T, u8>(value >> (8 * i) & mask).unwrap(),
            );
        }
    }

    fn write_register(&mut self, addr: u32, val: u8) {
        match addr {
            0x04000000..=0x0400005F => self.gpu.write_register(addr, val),
            0x04000060..=0x040000AF => self.apu.write_register(addr, val),
            0x040000B0..=0x040000BB => self.dma.channels[0].write(addr as u8 - 0xB0, val),
            0x040000BC..=0x040000C7 => self.dma.channels[1].write(addr as u8 - 0xBC, val),
            0x040000C8..=0x040000D3 => self.dma.channels[2].write(addr as u8 - 0xC8, val),
            0x040000D4..=0x040000DF => self.dma.channels[3].write(addr as u8 - 0xD4, val),
            0x040000E0..=0x040000FF => (),
            0x04000100..=0x04000103 => {
                self.timers.timers[0].write(&mut self.scheduler, addr as u8 % 4, val)
            }
            0x04000104..=0x04000107 => {
                self.timers.timers[1].write(&mut self.scheduler, addr as u8 % 4, val)
            }
            0x04000108..=0x0400010B => {
                self.timers.timers[2].write(&mut self.scheduler, addr as u8 % 4, val)
            }
            0x0400010C..=0x0400010F => {
                self.timers.timers[3].write(&mut self.scheduler, addr as u8 % 4, val)
            }
            0x04000110..=0x0400011F => (),
            0x04000120..=0x0400012B => warn!("Writng SerialCom(1) at {addr:08X} = {val:02X}",), // TODO: serial communication(1)
            0x0400012C..=0x0400012F => (),
            0x04000130..=0x04000133 => warn!("Writng Keypad at {addr:08X} = {val:02X}",), // TODO: Keypad Input
            0x04000134..=0x04000159 => warn!("Writng SerialCom(2) at {addr:08X} = {val:02X}",), // TODO: serial communication(2)
            0x0400015A..=0x040001FF => (),
            0x04000200 => self.interrupt_controller.enable.write::<0>(val),
            0x04000201 => self.interrupt_controller.enable.write::<1>(val),
            0x04000202 => self.interrupt_controller.request.write::<0>(val),
            0x04000203 => self.interrupt_controller.request.write::<1>(val),
            0x04000204 => self.waitcnt.write(&mut self.scheduler, 0, val),
            0x04000205 => self.waitcnt.write(&mut self.scheduler, 1, val),
            0x04000206..=0x04000207 => (), // Unused IO Register
            0x04000208 => self.interrupt_controller.master_enable.write::<0>(val),
            0x04000209 => self.interrupt_controller.master_enable.write::<1>(val),
            0x0400020A..=0x040002FF => (), // Unused IO Register
            0x04000300 => self.haltcnt = (self.haltcnt & !0x00FF) | val as u16,
            0x04000301 => self.haltcnt = (self.haltcnt & !0xFF00) | (val as u16) << 8,
            0x04000410 => (), // Undocumented
            0x04FFF600..=0x04FFF701 => self.mgba_test_suite.write_register(addr, val),
            0x04FFF780..=0x04FFF781 => self.mgba_test_suite.write_enable(addr, val),
            _ => (), //unreachable!("Writng Unimplemented IO Register at {addr:08X} = {val:02X}",),
        }
    }

    fn write_vram<T>(&mut self, addr: u32, value: T)
    where
        T: MemoryValue,
    {
        if size_of::<T>() == 1 {
            let addr = (addr & !0x1) as usize;
            let value = num::cast::<T, u8>(value).unwrap();
            self.gpu.vram[addr] = value;
            self.gpu.vram[addr + 1] = value;
        } else {
            Self::write_mem(&mut self.gpu.vram, addr, value);
        }
    }

    fn write_palette_ram<T>(&mut self, addr: u32, value: T)
    where
        T: MemoryValue,
    {
        if size_of::<T>() == 1 {
            let value = num::cast::<T, u8>(value).unwrap();
            self.gpu.write_palette_ram(addr & !0x1, value);
            self.gpu.write_palette_ram(addr | 0x1, value);
        } else {
            Self::write_from_bytes(&mut self.gpu, &Gpu::write_palette_ram, addr, value)
        }
    }

    fn write_oam<T>(&mut self, addr: u32, val: T)
    where
        T: MemoryValue,
    {
        if size_of::<T>() != 1 {
            Self::write_mem(&mut self.gpu.oam, addr, val)
        }
    }

    fn write_sram<T>(&mut self, addr: u32, val: T)
    where
        T: MemoryValue,
    {
        // TODO: this should do nothing if eeprom exists
        let addr = addr & 0x0EFFFFFF;
        let mask = FromPrimitive::from_u8(0xFF).unwrap();
        self.write_cart_backup(
            addr - 0x0E000000,
            num::cast::<T, u8>(val.rotate_right(addr * 8) & mask).unwrap(),
        );
    }

    fn write_cart_backup(&mut self, addr: u32, val: u8) {
        self.gamepak.write_save(addr, val)
    }
}

struct WaitStateControl {
    sram_setting: usize,
    n_wait_state_settings: [usize; 3],
    s_wait_state_settings: [usize; 3],
    phi_terminal_out: usize,
    use_prefetch: bool,
    _type_flag: bool,
    // Prefetch Buffer
    can_prefetch: bool,
    prefetch: VecDeque<u32>,
    prefetch_waitstate: usize,
    prefetch_addr: u32,
    prefetch_cycles_spent: u32,
}

impl WaitStateControl {
    const N_ACCESS_TIMINGS: [u32; 4] = [4, 3, 2, 8];
    const S_ACCESS_TIMINGS: [[u32; 2]; 3] = [[2, 1], [4, 1], [8, 1]];
    const SRAM_ACCESS_TIMINGS: [u32; 4] = [4, 3, 2, 8];

    pub fn new() -> Self {
        Self {
            sram_setting: 0,
            n_wait_state_settings: [0; 3],
            s_wait_state_settings: [0; 3],
            phi_terminal_out: 0,
            use_prefetch: false,
            _type_flag: false,
            // Prefetch Buffer
            can_prefetch: true,
            prefetch: VecDeque::new(),
            prefetch_waitstate: 0,
            prefetch_addr: 0x08000000,
            prefetch_cycles_spent: 0,
        }
    }

    pub fn get_rom_access_time(
        &mut self,
        wait_state: usize,
        cycle: Cycle,
        access_len: u32,
        addr: u32,
    ) -> u32 {
        assert_ne!(cycle, Cycle::I);
        assert!(access_len <= 2);
        let default_stall_time = match cycle {
            Cycle::N => WaitStateControl::N_ACCESS_TIMINGS[self.n_wait_state_settings[wait_state]],
            Cycle::S => {
                WaitStateControl::S_ACCESS_TIMINGS[wait_state]
                    [self.s_wait_state_settings[wait_state]]
            }
            Cycle::I => unreachable!(),
        };
        self.can_prefetch = false;
        let addr = addr & !0x1;
        let stall_time = if self.use_prefetch {
            if self.prefetch.contains(&addr) {
                0
            } else if self.prefetch_addr == addr {
                self.prefetch_addr = addr + 2;
                default_stall_time - self.prefetch_cycles_spent
            } else {
                self.prefetch_addr = addr + 2;
                default_stall_time
            }
        } else {
            default_stall_time
        };
        1 + if access_len == 2 {
            self.get_rom_access_time(wait_state, Cycle::S, 1, addr + 2)
        } else {
            0
        } + stall_time
    }

    pub fn clock_prefetch(&mut self, cycles: u32) {
        if self.use_prefetch && self.can_prefetch {
            for _ in 0..cycles {
                let prefetch_time = WaitStateControl::S_ACCESS_TIMINGS[self.prefetch_waitstate]
                    [self.s_wait_state_settings[self.prefetch_waitstate]];
                if self.prefetch_cycles_spent >= prefetch_time {
                    if self.prefetch.len() == 8 {
                        self.prefetch.pop_front();
                    }
                    assert!(self.prefetch.len() < 8);
                    self.prefetch.push_back(self.prefetch_addr);
                    self.prefetch_addr += 2;
                    self.prefetch_cycles_spent = 0;
                } else {
                    self.prefetch_cycles_spent += 1
                }
            }
        }
        self.can_prefetch = true;
    }

    pub fn get_sram_access_time(&self, cycle: Cycle) -> u32 {
        assert_ne!(cycle, Cycle::I);
        1 + WaitStateControl::SRAM_ACCESS_TIMINGS[self.sram_setting]
    }

    pub fn read(&self, byte: u8) -> u8 {
        match byte {
            0 => {
                (self.s_wait_state_settings[1] << 7
                    | self.n_wait_state_settings[1] << 5
                    | self.s_wait_state_settings[0] << 4
                    | self.n_wait_state_settings[0] << 2
                    | self.sram_setting) as u8
            }
            1 => {
                ((self._type_flag as usize) << 7
                    | (self.use_prefetch as usize) << 6
                    | self.phi_terminal_out << 3
                    | self.s_wait_state_settings[2] << 2
                    | self.n_wait_state_settings[2]) as u8
            }
            _ => unreachable!(),
        }
    }

    fn write(&mut self, _scheduler: &mut Scheduler, byte: u8, value: u8) {
        match byte {
            0 => {
                let value = value as usize;
                self.sram_setting = value & 0x3;
                self.n_wait_state_settings[0] = (value >> 2) & 0x3;
                self.s_wait_state_settings[0] = (value >> 4) & 0x1;
                self.n_wait_state_settings[1] = (value >> 5) & 0x3;
                self.s_wait_state_settings[1] = (value >> 7) & 0x1;
            }
            1 => {
                let value = value as usize;
                self.n_wait_state_settings[2] = value & 0x3;
                self.s_wait_state_settings[2] = (value >> 2) & 0x1;
                self.phi_terminal_out = (value >> 3) & 0x3;
                self.use_prefetch = (value >> 6) & 0x1 != 0;
                // Type Flag is read only
            }
            _ => unreachable!(),
        }
    }
}

mod mgba_test_suite {
    enum MGBALogLevel {
        Fatal,
        Error,
        Warn,
        Info,
        Debug,
    }

    impl MGBALogLevel {
        pub fn new(val: u16) -> Self {
            use MGBALogLevel::*;
            match val {
                0 => Fatal,
                1 => Error,
                2 => Warn,
                3 => Info,
                4 => Debug,
                _ => panic!("Invalid mGBA Log Level!"),
            }
        }
    }

    pub struct MGBATestSuite {
        buffer: [char; 0x100],
        // Registers
        enable: u16,
        flags: u16,
    }

    impl MGBATestSuite {
        pub fn new() -> MGBATestSuite {
            MGBATestSuite {
                buffer: ['\0'; 0x100],
                enable: 0,
                flags: 0,
            }
        }

        pub fn enabled(&self) -> bool {
            self.enable == 0xC0DE
        }

        pub fn write_enable(&mut self, addr: u32, value: u8) {
            match addr {
                0x4FFF780 => self.enable = self.enable & !0x00FF | (value as u16) & 0x00FF,
                0x4FFF781 => self.enable = self.enable & !0xFF00 | (value as u16) << 8 & 0xFF00,
                _ => (),
            }
        }

        pub fn read_register(&self, addr: u32) -> u8 {
            match addr {
                0x4FFF780 => {
                    if self.enabled() {
                        0xEA
                    } else {
                        0
                    }
                }
                0x4FFF781 => {
                    if self.enabled() {
                        0x1D
                    } else {
                        0
                    }
                }
                _ => 0,
            }
        }

        pub fn write_register(&mut self, addr: u32, value: u8) {
            if !self.enabled() {
                return;
            }
            match addr {
                0x4FFF600..=0x4FFF6FF => self.buffer[(addr - 0x4FFF600) as usize] = value as char,
                0x4FFF700 => self.flags = self.flags & !0x00FF | (value as u16) & 0x00FF,
                0x4FFF701 => {
                    self.flags = self.flags & !0xFF00 | (value as u16) << 8 & 0xFF00;
                    if self.flags & 0x100 != 0 {
                        use MGBALogLevel::*;
                        let null_byte_pos = self
                            .buffer
                            .iter()
                            .position(|&c| c == '\0')
                            .unwrap_or(self.buffer.len());
                        let message: String = self.buffer.iter().take(null_byte_pos).collect();

                        if message.contains("PASS") {
                            return;
                        }
                        let show_info = message.contains("FAIL")
                            && !message
                                .split(' ')
                                .skip(1)
                                .take(1)
                                .collect::<String>()
                                .contains('P');
                        let show_debug = !message
                            .split(' ')
                            .rev()
                            .take(1)
                            .collect::<String>()
                            .contains('P');
                        match MGBALogLevel::new(self.flags & 0x7) {
                            Fatal => error!("{message}"),
                            Error => error!("{message}"),
                            Warn => warn!("{message}"),
                            Info => {
                                if show_info {
                                    info!("{message}")
                                }
                            }
                            Debug => {
                                if show_debug {
                                    debug!("{message}")
                                }
                            }
                        }
                    }
                }
                _ => (),
            }
        }
    }
}
