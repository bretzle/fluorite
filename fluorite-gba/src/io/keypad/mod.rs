use bitflags::bitflags;
use fluorite_common::flume::Receiver;

bitflags! {
    pub struct KEYINPUT: u16 {
        const A = 1 << 0;
        const B = 1 << 1;
        const SELECT = 1 << 2;
        const START = 1 << 3;
        const RIGHT = 1 << 4;
        const LEFT = 1 << 5;
        const UP = 1 << 6;
        const DOWN = 1 << 7;
        const R = 1 << 8;
        const L = 1 << 9;
    }
}

impl KEYINPUT {
    pub fn read<const BYTE: u8>(&self) -> u8 {
        match BYTE {
            0 => self.bits as u8,
            1 => (self.bits >> 8) as u8,
            _ => unreachable!(),
        }
    }
}

bitflags! {
    pub struct KEYCNT: u16 {
        const A = 1 << 0;
        const B = 1 << 1;
        const SELECT = 1 << 2;
        const START = 1 << 3;
        const RIGHT = 1 << 4;
        const LEFT = 1 << 5;
        const UP = 1 << 6;
        const DOWN = 1 << 7;
        const R = 1 << 8;
        const L = 1 << 9;
        const IRQ_ENABLE = 1 << 14;
        const IRQ_COND_AND = 1 << 15;
    }
}

impl KEYCNT {
    pub fn read<const BYTE: u8>(&self) -> u8 {
        match BYTE {
            0 => self.bits as u8,
            1 => (self.bits >> 8) as u8,
            _ => unreachable!(),
        }
    }

    pub fn write(&mut self, byte: u8, value: u8) {
        match byte {
            0 => self.bits = self.bits & !0x00FF | (value as u16) & KEYCNT::all().bits,
            1 => self.bits = self.bits & !0xFF00 | (value as u16) << 8 & KEYCNT::all().bits,
            _ => unreachable!(),
        }
    }
}

pub struct Keypad {
    pub keyinput: KEYINPUT,
    pub keycnt: KEYCNT,
    rx: Receiver<(KEYINPUT, bool)>,
}

impl Keypad {
    pub fn new(rx: Receiver<(KEYINPUT, bool)>) -> Self {
        Self {
            keyinput: KEYINPUT::all(),
            keycnt: KEYCNT::empty(),
            rx,
        }
    }

    pub fn reset(&mut self) {
        self.keyinput = KEYINPUT::all();
        self.keycnt = KEYCNT::empty();
    }

    pub fn poll(&mut self) {
        for (key, pressed) in self.rx.try_iter() {
            if pressed {
                self.keyinput.remove(key);
            } else {
                self.keyinput.insert(key);
            }
        }
    }

    pub fn interrupt_requested(&self) -> bool {
        if self.keycnt.contains(KEYCNT::IRQ_ENABLE) {
            let irq_keys = self.keycnt - KEYCNT::IRQ_ENABLE - KEYCNT::IRQ_COND_AND;
            if self.keycnt.contains(KEYCNT::IRQ_COND_AND) {
                irq_keys.bits() & !self.keyinput.bits() == irq_keys.bits()
            } else {
                irq_keys.bits() & !self.keyinput.bits() != 0
            }
        } else {
            false
        }
    }
}
