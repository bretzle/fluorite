use std::{cell::Cell, rc::Rc};

use modular_bitfield::prelude::*;

pub type SharedInterruptFlags = Rc<Cell<IrqBitMask>>;

#[derive(Debug, Copy, Clone, PartialEq)]
#[allow(non_camel_case_types)]
pub enum Interrupt {
    LCD_VBlank = 0,
    LCD_HBlank = 1,
    LCD_VCounterMatch = 2,
    Timer0_Overflow = 3,
    Timer1_Overflow = 4,
    Timer2_Overflow = 5,
    Timer3_Overflow = 6,
    SerialCommunication = 7,
    DMA0 = 8,
    DMA1 = 9,
    DMA2 = 10,
    DMA3 = 11,
    Keypad = 12,
    GamePak = 13,
}

static_assertions::assert_eq_size!(IrqBitMask, u16);
#[bitfield]
#[repr(u16)]
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct IrqBitMask {
    pub LCD_VBlank: bool,
    pub LCD_HBlank: bool,
    pub LCD_VCounterMatch: bool,
    pub Timer0_Overflow: bool,
    pub Timer1_Overflow: bool,
    pub Timer2_Overflow: bool,
    pub Timer3_Overflow: bool,
    pub SerialCommunication: bool,
    pub DMA0: bool,
    pub DMA1: bool,
    pub DMA2: bool,
    pub DMA3: bool,
    pub Keypad: bool,
    pub GamePak: bool,
    #[skip]
    _reserved: modular_bitfield::prelude::B2,
}

pub struct InterruptController {
    pub master_enable: bool,
    enable: IrqBitMask,
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
}

pub fn signal_irq(interrupt_flags: &SharedInterruptFlags, i: Interrupt) {
    let _if = interrupt_flags.get();
    let new_if = u16::from(_if) | 1 << (i as usize);
    interrupt_flags.set(IrqBitMask::from(new_if));
}
