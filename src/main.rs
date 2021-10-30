use fluorite_arm::{cpu::Arm7tdmi, memory::MemoryInterface, Addr};
use fluorite_common::Shared;

// mod cpu;

const DATA: &[u8] = include_bytes!("../roms/first-1.bin");

struct SysBus {
    bios: Vec<u8>,
}

impl MemoryInterface for SysBus {
    fn load_8(&mut self, addr: Addr) -> u8 {
        self.bios[addr as usize]
    }

    fn load_16(&mut self, addr: Addr) -> u16 {
        let a = self.bios[addr as usize] as u16;
        let b = self.bios[addr as usize + 1] as u16;

        a | b << 8
    }

    fn load_32(&mut self, addr: Addr) -> u32 {
        let a = self.bios[addr as usize] as u32;
        let b = self.bios[addr as usize + 1] as u32;
        let c = self.bios[addr as usize + 2] as u32;
        let d = self.bios[addr as usize + 3] as u32;

        a | b << 8 | c << 16 | d << 24
    }

    fn store_8(&mut self, addr: Addr, val: u8) {
        self.bios[addr as usize] = val;
    }

    fn store_16(&mut self, addr: Addr, val: u16) {
        let a = (val & 0x00FF) as u8;
        let b = (val & 0xFF00 >> 8) as u8;

        self.bios[addr as usize] = a;
        self.bios[addr as usize + 1] = b;
    }

    fn store_32(&mut self, addr: Addr, val: u32) {
        let a = (val & 0x000000FF) as u8;
        let b = (val & 0x0000FF00 >> 8) as u8;
        let c = (val & 0x00FF0000 >> 16) as u8;
        let d = (val & 0xFF000000 >> 24) as u8;

        self.bios[addr as usize] = a;
        self.bios[addr as usize + 1] = b;
        self.bios[addr as usize + 2] = c;
        self.bios[addr as usize + 3] = d;
    }
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let bus = Shared::new(SysBus {
        bios: DATA.to_vec(),
    });
    let mut arm = Arm7tdmi::new(bus);

    loop {
        arm.step();
    }
}
