use fluorite_common::{bitfield, flume::Receiver};

bitfield! {
    #[derive(Clone, Copy)]
    pub struct KEYINPUT(u16) {
        raw: u16 @ ..,
        a: bool @ 0,
        b: bool @ 1,
        select: bool @ 2,
        start: bool @ 3,
        right: bool @ 4,
        left: bool @ 5,
        up: bool @ 6,
        down: bool @ 7,
        r: bool @ 8,
        l: bool @ 9,
    }
}

impl KEYINPUT {
    pub fn read<const BYTE: u8>(&self) -> u8 {
        match BYTE {
            0 => self.0 as u8,
            1 => (self.0 >> 8) as u8,
            _ => unreachable!(),
        }
    }
}

bitfield! {
    #[derive(Clone, Copy)]
    pub struct KEYCNT(u16) {
        raw: u16 @ ..,
        a: bool @ 0,
        b: bool @ 1,
        select: bool @ 2,
        start: bool @ 3,
        right: bool @ 4,
        left: bool @ 5,
        up: bool @ 6,
        down: bool @ 7,
        r: bool @ 8,
        l: bool @ 9,
        irq_enable: bool @ 14,
        irq_cond_and: bool @ 15,
    }
}

impl KEYCNT {
    pub fn read<const BYTE: u8>(&self) -> u8 {
        match BYTE {
            0 => self.0 as u8,
            1 => (self.0 >> 8) as u8,
            _ => unreachable!(),
        }
    }

    pub fn write(&mut self, byte: u8, value: u8) {
        match byte {
            0 => self.0 = self.0 & !0x00FF | (value as u16) & 0xC3FF,
            1 => self.0 = self.0 & !0xFF00 | (value as u16) << 8 & 0xC3FF,
            _ => unreachable!(),
        }
    }
}

pub struct Keypad {
    pub keyinput: KEYINPUT,
    pub keycnt: KEYCNT,
    rx: Receiver<(u16, bool)>,
}

impl Keypad {
    pub fn new(rx: Receiver<(u16, bool)>) -> Self {
        Self {
            keyinput: KEYINPUT(0x1FF),
            keycnt: KEYCNT(0),
            rx,
        }
    }

    pub fn reset(&mut self) {
        self.keyinput = KEYINPUT(0x1FF);
        self.keycnt = KEYCNT(0);
    }

    pub fn poll(&mut self) {
        for (key, pressed) in self.rx.try_iter() {
            if pressed {
                self.keyinput.0 &= !key;
            } else {
                self.keyinput.0 |= key;
            }
        }
    }

    pub fn interrupt_requested(&self) -> bool {
        if self.keycnt.irq_enable() {
            let irq_keys = self.keycnt.with_irq_enable(false).with_irq_cond_and(false);

            if self.keycnt.irq_cond_and() {
                irq_keys.raw() & !self.keyinput.raw() == irq_keys.raw()
            } else {
                irq_keys.raw() & !self.keyinput.raw() != 0
            }
        } else {
            false
        }
    }
}
