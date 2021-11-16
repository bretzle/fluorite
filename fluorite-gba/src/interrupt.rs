use std::{cell::Cell, rc::Rc};

use modular_bitfield::prelude::*;

pub type SharedInterruptFlags = Rc<Cell<IrqBitMask>>;

#[derive(Debug, Copy, Clone, PartialEq)]
#[allow(non_camel_case_types)]
pub enum Interrupt {
    LcdVBlank = 0,
    LcdHBlank = 1,
    LcdVCounterMatch = 2,
    Timer0Overflow = 3,
    Timer1Overflow = 4,
    Timer2Overflow = 5,
    Timer3Overflow = 6,
    SerialCommunication = 7,
    Dma0 = 8,
    Dma1 = 9,
    Dma2 = 10,
    Dma3 = 11,
    Keypad = 12,
    GamePak = 13,
}

impl Interrupt {
    pub fn from_usize(val: usize) -> Option<Self> {
        let ret = match val {
            0 => Interrupt::LcdVBlank,
            1 => Interrupt::LcdHBlank,
            2 => Interrupt::LcdVCounterMatch,
            3 => Interrupt::Timer0Overflow,
            4 => Interrupt::Timer1Overflow,
            5 => Interrupt::Timer2Overflow,
            6 => Interrupt::Timer3Overflow,
            7 => Interrupt::SerialCommunication,
            8 => Interrupt::Dma0,
            9 => Interrupt::Dma1,
            10 => Interrupt::Dma2,
            11 => Interrupt::Dma3,
            12 => Interrupt::Keypad,
            13 => Interrupt::GamePak,
            _ => return None,
        };
        Some(ret)
    }
}

static_assertions::assert_eq_size!(IrqBitMask, u16);
#[bitfield]
#[repr(u16)]
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct IrqBitMask {
    pub lcd_vblank: bool,
    pub lcd_hblank: bool,
    pub lcd_vcounter_match: bool,
    pub timer0_overflow: bool,
    pub timer1_overflow: bool,
    pub timer2_overflow: bool,
    pub timer3_overflow: bool,
    pub serial_communication: bool,
    pub dma0: bool,
    pub dma1: bool,
    pub dma2: bool,
    pub dma3: bool,
    pub keypad: bool,
    pub gamepak: bool,
    #[skip]
    _reserved: modular_bitfield::prelude::B2,
}

pub struct InterruptController {
    pub master_enable: bool,
    pub enable: IrqBitMask,
    flags: Rc<Cell<IrqBitMask>>,
}
impl InterruptController {
    pub fn new() -> Self {
        Self {
            master_enable: false,
            enable: IrqBitMask::default(),
            flags: Rc::new(Cell::new(IrqBitMask::default())),
        }
    }

    pub fn irq_pending(&self) -> bool {
        self.master_enable & ((u16::from(self.flags.get()) & u16::from(self.enable)) != 0)
    }

    pub fn clear(&mut self, value: u16) {
        let _if = self.flags.get();
        let new_if = u16::from(_if) & !value;
        self.flags.set(new_if.into());
    }
}

pub fn signal_irq(interrupt_flags: &SharedInterruptFlags, i: Interrupt) {
    let _if = interrupt_flags.get();
    let new_if = u16::from(_if) | 1 << (i as usize);
    interrupt_flags.set(IrqBitMask::from(new_if));
}