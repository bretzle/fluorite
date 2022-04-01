use super::Channel;
use std::collections::VecDeque;

pub struct DMASound {
    pub enable_right: u8,
    pub enable_left: u8,
    timer_select: u8,
    fifo: VecDeque<i8>,

    sample: i16,
}

impl DMASound {
    pub fn new() -> Self {
        Self {
            enable_right: 0,
            enable_left: 0,
            timer_select: 0,
            fifo: VecDeque::new(),
            sample: 0,
        }
    }

    pub fn on_timer_overflowed(&mut self, timer: usize) -> bool {
        if timer == self.timer_select as usize {
            self.sample = if let Some(sample) = self.fifo.pop_front() {
                sample as i16
            } else {
                0
            };
            self.fifo.len() <= 0x10
        } else {
            false
        }
    }

    pub fn read_cnt(&self) -> u8 {
        self.timer_select << 2 | self.enable_left << 1 | self.enable_right
    }

    pub fn write_cnt(&mut self, value: u8) {
        self.enable_right = value & 0x1;
        self.enable_left = value >> 1 & 0x1;
        self.timer_select = value >> 2 & 0x1;
        if value >> 3 & 0x1 != 0 {
            self.fifo.clear();
            self.sample = 0;
        }
    }

    pub fn write_fifo(&mut self, value: u8) {
        if self.fifo.len() < 0x20 {
            self.fifo.push_back(value as i8)
        }
    }
}

impl Channel for DMASound {
    fn generate_sample(&self) -> i16 {
        self.sample
    }

    fn is_on(&self) -> bool {
        false // Unused
    }
}
