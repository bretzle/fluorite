use fluorite_common::{ptr::WeakPointer, Event, Ram};
use registers::Registers;

mod instr_arm;
mod registers;

pub use registers::Reg;

include!(concat!(env!("OUT_DIR"), "/arm_lut.rs"));
// include!(concat!(env!("OUT_DIR"), "/thumb_lut.rs"));

#[derive(Clone, Copy)]
pub enum Access {
    Sequential,
    NonSequential,
}

pub struct Arm7tdmi<Bus> {
    bus: WeakPointer<Bus>,
    pub state: u32,

    pub regs: Registers,

    pipeline: [u32; 2],
    access: Access,
    target: u64,
    prefetch_active: u64,
    prefetch_cycles: u64,

    interrupt: Interrupt,
    waitcnt: (),
    haltcnt: (),
    postflg: (),

    ewram: Ram<{ 256 * 1024 }>,
    iwram: Ram<{ 32 * 1024 }>,
}

impl<Bus> Arm7tdmi<Bus>
where
    Bus: SysBus,
{
    pub fn new() -> Self {
        Self {
            bus: WeakPointer::default(),
            state: 0,
            regs: Registers::new(),
            pipeline: [0; 2],
            access: Access::NonSequential,
            target: 0,
            prefetch_active: 0,
            prefetch_cycles: 0,
            interrupt: Interrupt {
                delay: Event::new(|_| todo!()),
            },
            waitcnt: (),
            haltcnt: (),
            postflg: (),
            ewram: Ram::default(),
            iwram: Ram::default(),
        }
    }

    pub fn connect(&mut self, bus: WeakPointer<Bus>) {
        self.bus = bus;
    }

    pub fn reset(&mut self) {
        *self = Self {
            bus: self.bus.clone(),
            ..Self::new()
        }
    }

    pub fn init(&mut self) {
        self.flush_word();
        self.regs.pc += 4;
    }

    pub fn step_thumb(&mut self) {
        let instr = self.pipeline[0];

        self.pipeline[0] = self.pipeline[1];
        self.pipeline[1] = self.bus.read_half(self.regs.pc, self.access) as u32;

        let hash = (instr >> 6) as usize;

        // THUMB_LUT[hash](self, instr);

        self.regs.pc += self.regs.cpsr.size();
    }

    pub fn step_arm(&mut self) {
        let instr = self.pipeline[0];

        self.pipeline[0] = self.pipeline[1];
        self.pipeline[1] = self.bus.read_word(self.regs.pc, self.access);
        self.access = Access::Sequential;

        let hash = (((instr >> 16) & 0xFF0) | ((instr >> 4) & 0xF)) as usize;

        if self.regs.check(instr) {
            Self::ARM_LUT[hash](self, instr);
        }

        self.regs.pc += self.regs.cpsr.size();
    }

    fn flush_half(&mut self) {
        let pc = &mut self.regs.pc;
        *pc &= !0x1;
        self.pipeline[0] = self.bus.read_half(*pc + 0, Access::NonSequential) as u32;
        self.pipeline[1] = self.bus.read_half(*pc + 2, Access::Sequential) as u32;
        self.access = Access::Sequential;
        *pc += 2;
    }

    fn flush_word(&mut self) {
        let pc = &mut self.regs.pc;
        *pc &= !0x3;
        self.pipeline[0] = self.bus.read_word(*pc + 0, Access::NonSequential);
        self.pipeline[1] = self.bus.read_word(*pc + 4, Access::Sequential);
        self.access = Access::Sequential;
        *pc += 4;
    }
}

struct Interrupt {
    delay: Event,
    // enable: InterruptEnable,
    // request: InterruptRequest,
    // master: InterruptMaster,
}

pub trait SysBus {
    fn read_byte(&self, addr: u32, access: Access) -> u8;
    fn read_half(&self, addr: u32, access: Access) -> u16;
    fn read_word(&self, addr: u32, access: Access) -> u32;

    fn write_byte(&mut self, addr: u32, byte: u8, access: Access);
    fn write_half(&mut self, addr: u32, half: u16, access: Access);
    fn write_word(&mut self, addr: u32, word: u32, access: Access);
}
