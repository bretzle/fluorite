use crate::dma::DmaNotifier;
use crate::iodev::WaitControl;
use crate::{bios::Bios, cartridge::Cartridge, consts::*, iodev::IoDevices, sched::Scheduler};
use fluorite_arm::{
    memory::{MemoryAccess, MemoryAccessWidth, MemoryInterface},
    Addr,
};
use fluorite_common::Shared;

#[derive(Clone)]
pub struct SysBus {
    bios: Bios,
    ewram: Box<[u8]>,
    iwram: Box<[u8]>,
    cartridge: Cartridge,

    io: Shared<IoDevices>,
    scheduler: Shared<Scheduler>,

    cycle_luts: CycleLookupTables,
}

impl SysBus {
    pub fn new(
        bios: &[u8],
        rom: &[u8],
        scheduler: &Shared<Scheduler>,
        io: &Shared<IoDevices>,
    ) -> Self {
        let mut luts = CycleLookupTables::default();
        luts.init();
        luts.update_gamepak_waitstates(io.waitcnt);

        Self {
            bios: Bios::new(bios),
            ewram: vec![0; 256 * 1024].into_boxed_slice(),
            iwram: vec![0; 32 * 1024].into_boxed_slice(),
            cartridge: Cartridge::new(rom).unwrap(),
            io: io.clone(),
            scheduler: scheduler.clone(),
            cycle_luts: luts,
        }
    }

    fn read_invalid(&mut self, addr: Addr) -> u32 {
        panic!("invalid read @{:08X}", addr)
    }

    pub fn add_cycles(&mut self, addr: Addr, access: MemoryAccess, width: MemoryAccessWidth) {
        use MemoryAccess::*;
        use MemoryAccessWidth::*;
        let page = ((addr >> 24) & 0xF) as usize;

        let cycles = unsafe {
            match width {
                MemoryAccess8 | MemoryAccess16 => match access {
                    NonSeq => self.cycle_luts.n_cycles16.get_unchecked(page),
                    Seq => self.cycle_luts.s_cycles16.get_unchecked(page),
                },
                MemoryAccess32 => match access {
                    NonSeq => self.cycle_luts.n_cycles32.get_unchecked(page),
                    Seq => self.cycle_luts.s_cycles32.get_unchecked(page),
                },
            }
        };

        self.scheduler.update(*cycles);
    }
}

pub trait Bus {
    fn read_8(&mut self, addr: Addr) -> u8;
    fn read_16(&mut self, addr: Addr) -> u16 {
        self.read_8(addr) as u16 | (self.read_8(addr + 1) as u16) << 8
    }
    fn read_32(&mut self, addr: Addr) -> u32 {
        self.read_16(addr) as u32 | (self.read_16(addr + 2) as u32) << 16
    }

    fn write_8(&mut self, addr: Addr, val: u8);
    fn write_16(&mut self, addr: Addr, val: u16) {
        self.write_8(addr, (val & 0xFF) as u8);
        self.write_8(addr + 1, ((val >> 8) & 0xFF) as u8);
    }
    fn write_32(&mut self, addr: Addr, val: u32) {
        self.write_16(addr, (val & 0xffff) as u16);
        self.write_16(addr + 2, (val >> 16) as u16);
    }
}

impl Bus for SysBus {
    fn read_8(&mut self, addr: Addr) -> u8 {
        match addr & 0xff000000 {
            BIOS_ADDR => {
                if addr <= 0x3fff {
                    self.bios.read_8(addr)
                } else {
                    self.read_invalid(addr) as u8
                }
            }
            EWRAM_ADDR => self.ewram.read_8(addr & 0x3_ffff),
            IWRAM_ADDR => self.iwram.read_8(addr & 0x7fff),
            IOMEM_ADDR => {
                let addr = if addr & 0xffff == 0x8000 {
                    0x800
                } else {
                    addr & 0x00ffffff
                };
                self.io.read_8(addr)
            }
            PALRAM_ADDR | VRAM_ADDR | OAM_ADDR => self.io.gpu.read_8(addr),
            GAMEPAK_WS0_LO | GAMEPAK_WS0_HI | GAMEPAK_WS1_LO | GAMEPAK_WS1_HI | GAMEPAK_WS2_LO => {
                self.cartridge.read_8(addr)
            }
            GAMEPAK_WS2_HI => self.cartridge.read_8(addr),
            SRAM_LO | SRAM_HI => self.cartridge.read_8(addr),
            _ => self.read_invalid(addr) as u8,
        }
    }

    fn read_16(&mut self, addr: Addr) -> u16 {
        match addr & 0xFF000000 {
            BIOS_ADDR => {
                if addr <= 0x3FFE {
                    self.bios.read_16(addr)
                } else {
                    self.read_invalid(addr) as u16
                }
            }
            EWRAM_ADDR => self.ewram.read_16(addr & 0x3FFFE),
            IWRAM_ADDR => self.iwram.read_16(addr & 0x7FFE),
            IOMEM_ADDR => {
                let addr = if addr & 0xFFFE == 0x8000 {
                    0x800
                } else {
                    addr & 0xFFFFFE
                };
                self.io.read_16(addr)
            }
            PALRAM_ADDR | VRAM_ADDR | OAM_ADDR => self.io.gpu.read_16(addr),
            GAMEPAK_WS0_LO | GAMEPAK_WS0_HI | GAMEPAK_WS1_LO | GAMEPAK_WS1_HI | GAMEPAK_WS2_LO => {
                self.cartridge.read_16(addr)
            }
            GAMEPAK_WS2_HI => self.cartridge.read_16(addr),
            SRAM_LO | SRAM_HI => self.cartridge.read_16(addr),
            _ => self.read_invalid(addr) as u16,
        }
    }

    fn read_32(&mut self, addr: Addr) -> u32 {
        match addr & 0xFF000000 {
            BIOS_ADDR => {
                if addr <= 0x3FFC {
                    self.bios.read_32(addr)
                } else {
                    self.read_invalid(addr)
                }
            }
            EWRAM_ADDR => self.ewram.read_32(addr & 0x3_FFFC),
            IWRAM_ADDR => self.iwram.read_32(addr & 0x7FFC),
            IOMEM_ADDR => {
                todo!()
            }
            PALRAM_ADDR | VRAM_ADDR | OAM_ADDR => todo!(),
            GAMEPAK_WS0_LO | GAMEPAK_WS0_HI | GAMEPAK_WS1_LO | GAMEPAK_WS1_HI | GAMEPAK_WS2_LO => {
                self.cartridge.read_32(addr)
            }
            GAMEPAK_WS2_HI => self.cartridge.read_32(addr),
            SRAM_LO | SRAM_HI => self.cartridge.read_32(addr),
            _ => self.read_invalid(addr),
        }
    }

    fn write_8(&mut self, _addr: Addr, _val: u8) {
        todo!()
    }

    fn write_16(&mut self, addr: Addr, val: u16) {
        match addr & 0xFF000000 {
            BIOS_ADDR => {}
            EWRAM_ADDR => self.ewram.write_16(addr & 0x3_FFFE, val),
            IWRAM_ADDR => self.iwram.write_16(addr & 0x7FFE, val),
            IOMEM_ADDR => {
                let addr = if addr & 0xFFFE == 0x8000 {
                    0x800
                } else {
                    addr & 0x00FFFFFE
                };
                self.io.write_16(addr, val)
            }
            PALRAM_ADDR | VRAM_ADDR | OAM_ADDR => self.io.gpu.write_16(addr, val),
            GAMEPAK_WS0_LO => self.cartridge.write_16(addr, val),
            GAMEPAK_WS2_HI => self.cartridge.write_16(addr, val),
            SRAM_LO | SRAM_HI => self.cartridge.write_16(addr, val),
            _ => {
                println!("trying to write invalid address {:#x}", addr);
            }
        }
    }

    fn write_32(&mut self, addr: Addr, val: u32) {
        match addr & 0xFF000000 {
            BIOS_ADDR => {}
            EWRAM_ADDR => self.ewram.write_32(addr & 0x3_FFFC, val),
            IWRAM_ADDR => self.iwram.write_32(addr & 0x7FFC, val),
            IOMEM_ADDR => {
                let addr = if addr & 0xFFFC == 0x8000 {
                    0x800
                } else {
                    addr & 0x00FFFFFC
                };
                self.io.write_32(addr, val)
            }
            PALRAM_ADDR | VRAM_ADDR | OAM_ADDR => self.io.gpu.write_32(addr, val),
            GAMEPAK_WS0_LO => self.cartridge.write_32(addr, val),
            GAMEPAK_WS2_HI => self.cartridge.write_32(addr, val),
            SRAM_LO | SRAM_HI => self.cartridge.write_32(addr, val),
            _ => {
                // warn!("trying to write invalid address {:#x}", addr);
                // TODO open bus
            }
        }
    }
}

impl MemoryInterface for SysBus {
    fn load_8(&mut self, addr: Addr, access: MemoryAccess) -> u8 {
        self.add_cycles(addr, access, MemoryAccessWidth::MemoryAccess8);
        self.read_8(addr)
    }

    fn load_16(&mut self, addr: Addr, access: MemoryAccess) -> u16 {
        self.add_cycles(addr, access, MemoryAccessWidth::MemoryAccess16);
        self.read_16(addr)
    }

    fn load_32(&mut self, addr: Addr, access: MemoryAccess) -> u32 {
        self.add_cycles(addr, access, MemoryAccessWidth::MemoryAccess32);
        self.read_32(addr)
    }

    fn store_8(&mut self, addr: Addr, value: u8, access: MemoryAccess) {
        self.add_cycles(addr, access, MemoryAccessWidth::MemoryAccess8);
        self.write_8(addr, value);
    }

    fn store_16(&mut self, addr: Addr, value: u16, access: MemoryAccess) {
        self.add_cycles(addr, access, MemoryAccessWidth::MemoryAccess8);
        self.write_16(addr, value);
    }

    fn store_32(&mut self, addr: Addr, value: u32, access: MemoryAccess) {
        self.add_cycles(addr, access, MemoryAccessWidth::MemoryAccess8);
        self.write_32(addr, value);
    }

    fn idle_cycle(&mut self) {
        self.scheduler.update(1)
    }
}

impl Bus for Box<[u8]> {
    fn read_8(&mut self, addr: Addr) -> u8 {
        self[addr as usize]
    }

    fn write_8(&mut self, addr: Addr, val: u8) {
        self[addr as usize] = val;
    }
}

impl DmaNotifier for SysBus {
    fn notify(&mut self, _timing: u16) {
        // TODO
    }
}

#[derive(Clone)]
struct CycleLookupTables {
    n_cycles32: Box<[usize]>,
    s_cycles32: Box<[usize]>,
    n_cycles16: Box<[usize]>,
    s_cycles16: Box<[usize]>,
}

const CYCLE_LUT_SIZE: usize = 0x100;

impl Default for CycleLookupTables {
    fn default() -> CycleLookupTables {
        CycleLookupTables {
            n_cycles32: vec![1; CYCLE_LUT_SIZE].into_boxed_slice(),
            s_cycles32: vec![1; CYCLE_LUT_SIZE].into_boxed_slice(),
            n_cycles16: vec![1; CYCLE_LUT_SIZE].into_boxed_slice(),
            s_cycles16: vec![1; CYCLE_LUT_SIZE].into_boxed_slice(),
        }
    }
}

impl CycleLookupTables {
    pub fn init(&mut self) {
        self.n_cycles32[PAGE_EWRAM] = 6;
        self.s_cycles32[PAGE_EWRAM] = 6;
        self.n_cycles16[PAGE_EWRAM] = 3;
        self.s_cycles16[PAGE_EWRAM] = 3;

        self.n_cycles32[PAGE_OAM] = 2;
        self.s_cycles32[PAGE_OAM] = 2;
        self.n_cycles16[PAGE_OAM] = 1;
        self.s_cycles16[PAGE_OAM] = 1;

        self.n_cycles32[PAGE_VRAM] = 2;
        self.s_cycles32[PAGE_VRAM] = 2;
        self.n_cycles16[PAGE_VRAM] = 1;
        self.s_cycles16[PAGE_VRAM] = 1;

        self.n_cycles32[PAGE_PALRAM] = 2;
        self.s_cycles32[PAGE_PALRAM] = 2;
        self.n_cycles16[PAGE_PALRAM] = 1;
        self.s_cycles16[PAGE_PALRAM] = 1;
    }

    pub fn update_gamepak_waitstates(&mut self, waitcnt: WaitControl) {
        static S_GAMEPAK_NSEQ_CYCLES: [usize; 4] = [4, 3, 2, 8];
        static S_GAMEPAK_WS0_SEQ_CYCLES: [usize; 2] = [2, 1];
        static S_GAMEPAK_WS1_SEQ_CYCLES: [usize; 2] = [4, 1];
        static S_GAMEPAK_WS2_SEQ_CYCLES: [usize; 2] = [8, 1];

        let ws0_first_access = waitcnt.ws0_first_access() as usize;
        let ws1_first_access = waitcnt.ws1_first_access() as usize;
        let ws2_first_access = waitcnt.ws2_first_access() as usize;
        let ws0_second_access = waitcnt.ws0_second_access() as usize;
        let ws1_second_access = waitcnt.ws1_second_access() as usize;
        let ws2_second_access = waitcnt.ws2_second_access() as usize;

        // update SRAM access
        let sram_wait_cycles = 1 + S_GAMEPAK_NSEQ_CYCLES[waitcnt.sram_wait_control() as usize];
        self.n_cycles32[PAGE_SRAM_LO] = sram_wait_cycles;
        self.n_cycles32[PAGE_SRAM_LO] = sram_wait_cycles;
        self.n_cycles16[PAGE_SRAM_HI] = sram_wait_cycles;
        self.n_cycles16[PAGE_SRAM_HI] = sram_wait_cycles;
        self.s_cycles32[PAGE_SRAM_LO] = sram_wait_cycles;
        self.s_cycles32[PAGE_SRAM_LO] = sram_wait_cycles;
        self.s_cycles16[PAGE_SRAM_HI] = sram_wait_cycles;
        self.s_cycles16[PAGE_SRAM_HI] = sram_wait_cycles;

        // update both pages of each waitstate
        for i in 0..2 {
            self.n_cycles16[PAGE_GAMEPAK_WS0 + i] = 1 + S_GAMEPAK_NSEQ_CYCLES[ws0_first_access];
            self.s_cycles16[PAGE_GAMEPAK_WS0 + i] = 1 + S_GAMEPAK_WS0_SEQ_CYCLES[ws0_second_access];

            self.n_cycles16[PAGE_GAMEPAK_WS1 + i] = 1 + S_GAMEPAK_NSEQ_CYCLES[ws1_first_access];
            self.s_cycles16[PAGE_GAMEPAK_WS1 + i] = 1 + S_GAMEPAK_WS1_SEQ_CYCLES[ws1_second_access];

            self.n_cycles16[PAGE_GAMEPAK_WS2 + i] = 1 + S_GAMEPAK_NSEQ_CYCLES[ws2_first_access];
            self.s_cycles16[PAGE_GAMEPAK_WS2 + i] = 1 + S_GAMEPAK_WS2_SEQ_CYCLES[ws2_second_access];

            // ROM 32bit accesses are split into two 16bit accesses 1N+1S
            self.n_cycles32[PAGE_GAMEPAK_WS0 + i] =
                self.n_cycles16[PAGE_GAMEPAK_WS0 + i] + self.s_cycles16[PAGE_GAMEPAK_WS0 + i];
            self.n_cycles32[PAGE_GAMEPAK_WS1 + i] =
                self.n_cycles16[PAGE_GAMEPAK_WS1 + i] + self.s_cycles16[PAGE_GAMEPAK_WS1 + i];
            self.n_cycles32[PAGE_GAMEPAK_WS2 + i] =
                self.n_cycles16[PAGE_GAMEPAK_WS2 + i] + self.s_cycles16[PAGE_GAMEPAK_WS2 + i];

            self.s_cycles32[PAGE_GAMEPAK_WS0 + i] = 2 * self.s_cycles16[PAGE_GAMEPAK_WS0 + i];
            self.s_cycles32[PAGE_GAMEPAK_WS1 + i] = 2 * self.s_cycles16[PAGE_GAMEPAK_WS1 + i];
            self.s_cycles32[PAGE_GAMEPAK_WS2 + i] = 2 * self.s_cycles16[PAGE_GAMEPAK_WS2 + i];
        }
    }
}
