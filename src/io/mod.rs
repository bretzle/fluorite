use self::{
    gpu::Gpu,
    memory::{MemoryRegion, MemoryValue},
    scheduler::Scheduler,
};
use crate::gba::{DebugSpec, Pixels};
use num::cast::FromPrimitive;
use std::{cell::Cell, collections::VecDeque, mem::size_of};

pub mod gpu;
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
    apu: (),
    dma: (),
    timers: (),
    keypad: (),
    interrupt_controller: (),
    rtc: (),
    backup: (),

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
            apu: (),
            dma: (),
            timers: (),
            keypad: (),
            interrupt_controller: (),
            rtc: (),
            backup: (),

            haltcnt: 0,
            waitcnt: WaitStateControl::new(),

            pc: 0,
            in_thumb: false,
            pipeline: [0; 2],
            bios_latch: Cell::new(0),
        };

        (bus, pixels, debug)
    }

    pub fn read<T>(&self, addr: u32) -> T
    where
        T: MemoryValue,
    {
        use num::cast;

        fn read_from_bytes<T, F, D>(device: &D, read_fn: &F, addr: u32) -> T
        where
            T: MemoryValue,
            F: Fn(&D, u32) -> u8,
        {
            let mut value: T = num::zero();
            for i in 0..size_of::<T>() as u32 {
                value =
                    cast::<u8, T>(read_fn(device, addr + i)).unwrap() << (8 * i as usize) | value;
            }
            value
        }

        match MemoryRegion::get_region(addr) {
            MemoryRegion::BIOS => self.read_bios(addr),
            MemoryRegion::EWRAM => todo!(),
            MemoryRegion::IWRAM => todo!(),
            MemoryRegion::IO => todo!(),
            MemoryRegion::Palette => todo!(),
            MemoryRegion::VRAM => todo!(),
            MemoryRegion::OAM => todo!(),
            MemoryRegion::ROM0L => {
                // if (0x080000C4..=0x80000C9).contains(&addr) {
                // 	todo!()
                // } else {
                self.read_rom(addr)
                // }
            }
            MemoryRegion::ROM0H => todo!(),
            MemoryRegion::ROM1L => todo!(),
            MemoryRegion::ROM1H => todo!(),
            MemoryRegion::ROM2L => todo!(),
            MemoryRegion::ROM2H => todo!(),
            MemoryRegion::SRAM => todo!(),
            MemoryRegion::Unused => todo!(),
        }
    }

    pub fn write<T>(&mut self, addr: u32, value: T)
    where
        T: MemoryValue,
    {
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
                    num::cast::<T, u8>(value >> 8 * i & mask).unwrap(),
                );
            }
        }

        match MemoryRegion::get_region(addr) {
            MemoryRegion::BIOS => todo!(),
            MemoryRegion::EWRAM => todo!(),
            MemoryRegion::IWRAM => todo!(),
            MemoryRegion::IO => write_from_bytes(self, &Self::write_register, addr, value),
            MemoryRegion::Palette => todo!(),
            MemoryRegion::VRAM => self.write_vram(Gpu::parse_vram_addr(addr), value),
            MemoryRegion::OAM => todo!(),
            MemoryRegion::ROM0L => todo!(),
            MemoryRegion::ROM0H => todo!(),
            MemoryRegion::ROM1L => todo!(),
            MemoryRegion::ROM1H => todo!(),
            MemoryRegion::ROM2L => todo!(),
            MemoryRegion::ROM2H => todo!(),
            MemoryRegion::SRAM => todo!(),
            MemoryRegion::Unused => todo!(),
        }
    }

    pub fn inc_clock<C: Into<Cycle>>(&mut self, cycle: C, addr: u32, access_width: u32) {
        let cycle = cycle.into();
        let clocks_inc = if cycle == Cycle::I {
            1
        } else {
            match MemoryRegion::get_region(addr) {
                MemoryRegion::BIOS => 1,                                 // BIOS ROM
                MemoryRegion::EWRAM => [3, 3, 6][access_width as usize], // WRAM - On-board 256K
                MemoryRegion::IWRAM => 1,
                MemoryRegion::IO => 1,
                MemoryRegion::Palette => {
                    if access_width < 2 {
                        1
                    } else {
                        2
                    }
                }
                MemoryRegion::VRAM => {
                    if access_width < 2 {
                        1
                    } else {
                        2
                    }
                }
                MemoryRegion::OAM => 1,
                MemoryRegion::ROM0L | MemoryRegion::ROM0H => {
                    self.waitcnt
                        .get_rom_access_time(0, cycle, access_width, addr)
                }
                MemoryRegion::ROM1L | MemoryRegion::ROM1H => {
                    self.waitcnt
                        .get_rom_access_time(1, cycle, access_width, addr)
                }
                MemoryRegion::ROM2L | MemoryRegion::ROM2H => {
                    self.waitcnt
                        .get_rom_access_time(2, cycle, access_width, addr)
                }
                MemoryRegion::SRAM => self.waitcnt.get_sram_access_time(cycle),
                MemoryRegion::Unused => 1,
            }
        };
        self.waitcnt.clock_prefetch(clocks_inc);

        for _ in 0..clocks_inc {
            // TODO: self.handle_events();
            // TODO: self.rtc.clock();
            // TODO: self.apu.clock();
        }
        self.clocks_ahead += clocks_inc;
        while self.clocks_ahead >= 4 {
            self.clocks_ahead -= 4;
            // TODO: self.interrupt_controller.request |= self.ppu.emulate_dot();
        }
    }

    pub fn get_cycle(&self) -> usize {
        self.scheduler.cycle
    }

    pub fn poll_keypad_updates(&mut self) {
        // TODO
    }

    pub fn run_dma(&mut self) {
        // TODO
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
            println!("Returning Invalid ROM Read at 0x{:08X}", addr + 0x08000000);
            num::zero()
        }
    }
}

impl Sysbus {
    fn write_register(&mut self, addr: u32, val: u8) {
        match addr {
            0x04000000..=0x0400005F => self.gpu.write_register(&mut self.scheduler, addr, val),
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
}

struct WaitStateControl {
    sram_setting: usize,
    n_wait_state_settings: [usize; 3],
    s_wait_state_settings: [usize; 3],
    phi_terminal_out: usize,
    use_prefetch: bool,
    type_flag: bool,
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
            type_flag: false,
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
}
