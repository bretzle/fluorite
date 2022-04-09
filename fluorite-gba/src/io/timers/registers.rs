#[derive(Clone, Copy)]
pub struct TmCnt {
    pub prescaler: u8,
    pub count_up: bool,
    pub irq: bool,
    pub start: bool,
}

impl TmCnt {
    pub fn new() -> TmCnt {
        TmCnt {
            prescaler: 0,
            count_up: false,
            irq: false,
            start: false,
        }
    }

    pub fn read(&self, byte: u8) -> u8 {
        match byte {
            0 => {
                (self.start as u8) << 7
                    | (self.irq as u8) << 6
                    | (self.count_up as u8) << 2
                    | self.prescaler
            }
            1 => 0,
            _ => unreachable!(),
        }
    }

    pub fn write<const BYTE: u8>(&mut self, value: u8) {
        match BYTE {
            0 => {
                self.start = value >> 7 & 0x1 != 0;
                self.irq = value >> 6 & 0x1 != 0;
                self.count_up = value >> 2 & 0x1 != 0;
                self.prescaler = value & 0x3;
            }
            1 => (),
            _ => unreachable!(),
        }
    }
}
