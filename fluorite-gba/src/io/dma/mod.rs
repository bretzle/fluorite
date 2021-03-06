use self::registers::{Address, DmaCnt, WordCount};

mod registers;

pub struct Dma {
    pub channels: [DmaChannel; 4],
    pub in_dma: bool,
}

impl Dma {
    pub fn new() -> Self {
        Self {
            channels: [
                DmaChannel::new(0, false, false, false),
                DmaChannel::new(1, true, true, false),
                DmaChannel::new(2, true, false, false),
                DmaChannel::new(3, true, true, true),
            ],
            in_dma: false,
        }
    }

    pub fn get_channel_running(
        &mut self,
        hblank_called: bool,
        vblank_called: bool,
        fifo_req: [bool; 2],
    ) -> usize {
        for (i, channel) in self.channels.iter().enumerate() {
            if (*channel).needs_to_transfer(hblank_called, vblank_called, fifo_req) {
                return i;
            }
        }

        4
    }
}

pub struct DmaChannel {
    pub num: usize,
    pub sad_latch: u32,
    pub dad_latch: u32,
    pub count_latch: u32,

    sad: Address,
    dad: Address,
    pub count: WordCount,
    pub cnt: DmaCnt,
}

impl DmaChannel {
    const FIFO_A_ADDR: u32 = 0x40000A0;
    const FIFO_B_ADDR: u32 = 0x40000A4;

    pub fn new(
        num: usize,
        src_any_memory: bool,
        dest_any_memory: bool,
        count_is16bit: bool,
    ) -> DmaChannel {
        DmaChannel {
            num,
            sad_latch: 0,
            dad_latch: 0,
            count_latch: 0,

            sad: Address::new(src_any_memory),
            dad: Address::new(dest_any_memory),
            count: WordCount::new(count_is16bit),
            cnt: DmaCnt::new(count_is16bit),
        }
    }

    pub fn needs_to_transfer(
        &self,
        hblank_called: bool,
        vblank_called: bool,
        fifo_req: [bool; 2],
    ) -> bool {
        if !self.cnt.enable {
            return false;
        }
        match self.cnt.start_timing {
            0 => true,
            1 => vblank_called,
            2 => hblank_called,
            3 => match self.num {
                0 => {
                    println!("Special DMA for DMA 0 Called!");
                    false
                }
                1 | 2 => {
                    fifo_req[0] && self.dad.addr == DmaChannel::FIFO_A_ADDR
                        || fifo_req[1] && self.dad.addr == DmaChannel::FIFO_B_ADDR
                }
                3 => {
                    println!("Video Capture DMA Called!");
                    false
                } // TODO: Implement Video Capture DMA
                _ => unreachable!(),
            },
            _ => unreachable!(),
        }
    }

    pub fn latch(&mut self) {
        self.sad_latch = self.sad.addr;
        self.dad_latch = self.dad.addr;
        self.count_latch = if self.count.count == 0 {
            self.count.get_max() + 1
        } else {
            self.count.count as u32
        };
    }

    pub fn read(&self, byte: u8) -> u8 {
        match byte {
            0x0 => self.sad.read(0),
            0x1 => self.sad.read(1),
            0x2 => self.sad.read(2),
            0x3 => self.sad.read(3),
            0x4 => self.dad.read(0),
            0x5 => self.dad.read(1),
            0x6 => self.dad.read(2),
            0x7 => self.dad.read(3),
            0x8 => self.count.read(0),
            0x9 => self.count.read(1),
            0xA => self.cnt.read::<0>(),
            0xB => self.cnt.read::<1>(),
            _ => unreachable!(),
        }
    }

    pub fn write(&mut self, byte: u8, val: u8) {
        match byte {
            0x0 => self.sad.write::<0>(val),
            0x1 => self.sad.write::<1>(val),
            0x2 => self.sad.write::<2>(val),
            0x3 => self.sad.write::<3>(val),
            0x4 => self.dad.write::<0>(val),
            0x5 => self.dad.write::<1>(val),
            0x6 => self.dad.write::<2>(val),
            0x7 => self.dad.write::<3>(val),
            0x8 => self.count.write::<0>(val),
            0x9 => self.count.write::<1>(val),
            0xA => self.cnt.write::<0>(val),
            0xB => {
                let prev_enable = self.cnt.enable;
                self.cnt.write::<1>(val);
                if !prev_enable && self.cnt.enable {
                    self.latch()
                }
            }
            _ => unreachable!(),
        }
    }
}
