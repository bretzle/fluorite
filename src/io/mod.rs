use self::{
    dma::Dma,
    gpu::Gpu,
    interrupt_controller::InterruptController,
    memory::{MemoryRegion, MemoryValue},
    scheduler::{Event, EventType, Scheduler},
};
use crate::{
    gba::{self, DebugSpec, Pixels},
    io::interrupt_controller::InterruptRequest,
};
use num::cast::FromPrimitive;
use std::{cell::Cell, collections::VecDeque, mem::size_of};

pub mod dma;
pub mod gpu;
pub mod interrupt_controller;
pub mod keypad;
pub mod memory;
pub mod scheduler;

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
    bios: Box<[u8]>,
    rom: Box<[u8]>,

    ewram: Box<[u8]>,
    iwram: Box<[u8]>,

    scheduler: Scheduler,
    clocks_ahead: u32,

    // Devices
    gpu: Gpu,
    _apu: (),
    dma: Dma,
    _timers: (),
    _keypad: (),
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
}

impl Sysbus {
    const EWRAM_MASK: u32 = 0x3FFFF;
    const IWRAM_MASK: u32 = 0x7FFF;

    pub fn new(bios: Vec<u8>, rom: Vec<u8>) -> (Self, Pixels, DebugSpec) {
        let (gpu, pixels, debug) = Gpu::new();

        let bus = Self {
            bios: bios.into_boxed_slice(),
            rom: rom.into_boxed_slice(),

            ewram: vec![0; 0x40000].into_boxed_slice(),
            iwram: vec![0; 0x8000].into_boxed_slice(),

            scheduler: Scheduler::new(),
            clocks_ahead: 0,

            gpu,
            _apu: (),
            dma: Dma::new(),
            _timers: (),
            _keypad: (),
            interrupt_controller: InterruptController::new(),
            _rtc: (),
            _backup: (),

            haltcnt: 0,
            waitcnt: WaitStateControl::new(),

            pc: 0,
            in_thumb: false,
            pipeline: [0; 2],
            bios_latch: Cell::new(0xE129F000),
        };

        (bus, pixels, debug)
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
            MemoryRegion::Palette => todo!(),
            MemoryRegion::Vram => todo!(),
            MemoryRegion::Oam => todo!(),
            MemoryRegion::Rom0L => {
                // if (0x080000C4..=0x80000C9).contains(&addr) {
                // 	todo!()
                // } else {
                self.read_rom(addr)
                // }
            }
            MemoryRegion::Rom0H => todo!(),
            MemoryRegion::Rom1L => todo!(),
            MemoryRegion::Rom1H => todo!(),
            MemoryRegion::Rom2L => todo!(),
            MemoryRegion::Rom2H => todo!(),
            MemoryRegion::Sram => todo!(),
            MemoryRegion::Unused => self.read_openbus(addr),
        }
    }

    pub fn write<T>(&mut self, addr: u32, value: T)
    where
        T: MemoryValue,
    {
        match MemoryRegion::get_region(addr) {
            MemoryRegion::Bios => todo!(),
            MemoryRegion::Ewram => Self::write_mem(&mut self.ewram, addr & Self::EWRAM_MASK, value),
            MemoryRegion::Iwram => Self::write_mem(&mut self.iwram, addr & Self::IWRAM_MASK, value),
            MemoryRegion::Io => Self::write_from_bytes(self, &Self::write_register, addr, value),
            MemoryRegion::Palette => self.write_palette_ram(addr, value),
            MemoryRegion::Vram => self.write_vram(Gpu::parse_vram_addr(addr), value),
            MemoryRegion::Oam => todo!(),
            MemoryRegion::Rom0L => todo!(),
            MemoryRegion::Rom0H => todo!(),
            MemoryRegion::Rom1L => todo!(),
            MemoryRegion::Rom1H => todo!(),
            MemoryRegion::Rom2L => todo!(),
            MemoryRegion::Rom2H => todo!(),
            MemoryRegion::Sram => todo!(),
            MemoryRegion::Unused => todo!(),
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
            // TODO: self.rtc.clock();
            // TODO: self.apu.clock();
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
            EventType::_TimerOverflow(_timer) => println!("TODO: {event:?}"),
            EventType::FrameSequencer(step) => {
                // self.apu.clock_sequencer(step);
                self.scheduler.add(Event {
                    cycle: self.scheduler.cycle + (gba::CLOCK_FREQ / 512),
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
        // TODO
    }

    pub fn run_dma(&mut self) {
        let dma_channel = self.dma.get_channel_running(
            self.gpu.hblank_called(),
            self.gpu.vblank_called(),
            // [self.apu.fifo_a_req(), self.apu.fifo_b_req()],
            [false; 2],
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
            println!(
                "Running DMA{}: Writing {} values to {:08X} from {:08X}, size: {}",
                dma_channel,
                count,
                dest_addr,
                src_addr,
                if transfer_32 { 32 } else { 16 }
            );

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
            self.bios_latch.set(Self::read_mem(&self.bios, addr)); // Always 32 bit read
            Self::read_mem(&self.bios, addr)
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
        if (addr as usize) < self.rom.len() {
            Self::read_mem(&self.rom, addr)
        } else {
            num::zero()
        }
    }

    fn read_io_register(&self, addr: u32) -> u8 {
        match addr {
            0x04000000..=0x0400005F => self.gpu.read_register(addr),
            _ => panic!("Reading Unimplemented IO Register at {addr:08X}"),
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
            0x04000000..=0x0400005F => self.gpu.write_register(&mut self.scheduler, addr, val),
            0x040000B0..=0x040000BB => {
                self.dma.channels[0].write(&mut self.scheduler, addr as u8 - 0xB0, val)
            }
            0x040000BC..=0x040000C7 => {
                self.dma.channels[1].write(&mut self.scheduler, addr as u8 - 0xBC, val)
            }
            0x040000C8..=0x040000D3 => {
                self.dma.channels[2].write(&mut self.scheduler, addr as u8 - 0xC8, val)
            }
            0x040000D4..=0x040000DF => {
                self.dma.channels[3].write(&mut self.scheduler, addr as u8 - 0xD4, val)
            }
            0x04000110..=0x0400011F => (), // unused. TODO: verify this
            0x04000120..=0x0400012A => println!("Writng SerialCom(1) at {addr:08X} = {val:02X}",), // TODO: serial communication(1)
            0x04000130..=0x04000132 => println!("Writng Keypad at {addr:08X} = {val:02X}",), // TODO: Keypad Input
            0x04000134..=0x04000158 => println!("Writng SerialCom(2) at {addr:08X} = {val:02X}",), // TODO: serial communication(2)
            0x04000200 => self
                .interrupt_controller
                .enable
                .write(&mut self.scheduler, 0, val),
            0x04000201 => self
                .interrupt_controller
                .enable
                .write(&mut self.scheduler, 1, val),
            0x04000202 => self
                .interrupt_controller
                .request
                .write(&mut self.scheduler, 0, val),
            0x04000203 => self
                .interrupt_controller
                .request
                .write(&mut self.scheduler, 1, val),
            0x04000204 => self.waitcnt.write(&mut self.scheduler, 0, val),
            0x04000205 => self.waitcnt.write(&mut self.scheduler, 1, val),
            0x04000206..=0x04000207 => (), // Unused IO Register
            0x04000208 => {
                self.interrupt_controller
                    .master_enable
                    .write(&mut self.scheduler, 0, val)
            }
            0x04000209 => {
                self.interrupt_controller
                    .master_enable
                    .write(&mut self.scheduler, 1, val)
            }
            0x0400020A..=0x040002FF => (), // Unused IO Register
            0x04000300 => self.haltcnt = (self.haltcnt & !0x00FF) | val as u16,
            0x04000301 => self.haltcnt = (self.haltcnt & !0xFF00) | (val as u16) << 8,
            _ => panic!("Writng Unimplemented IO Register at {addr:08X} = {val:02X}",),
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

    fn _read(&self, byte: u8) -> u8 {
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
