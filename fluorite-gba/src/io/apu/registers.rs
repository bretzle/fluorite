pub struct SoundEnableFlags {
    pub channel1: u8,
    pub channel2: u8,
    pub channel3: u8,
    pub channel4: u8,
}

impl SoundEnableFlags {
    pub fn new() -> Self {
        Self {
            channel4: 0,
            channel3: 0,
            channel2: 0,
            channel1: 0,
        }
    }

    fn read(&self) -> u8 {
        self.channel4 << 3 | self.channel3 << 2 | self.channel2 << 1 | self.channel1
    }

    pub fn write(&mut self, value: u8) {
        self.channel1 = value & 0x1;
        self.channel2 = value >> 1 & 0x1;
        self.channel3 = value >> 2 & 0x1;
        self.channel4 = value >> 3 & 0x1;
    }
}

pub struct SoundCnt {
    pub psg_master_volume_r: u8,
    pub psg_master_volume_l: u8,
    pub psg_enable_r: SoundEnableFlags,
    pub psg_enable_l: SoundEnableFlags,
    pub psg_volume: u8,
    pub dma_sound_a_vol: u8,
    pub dma_sound_b_vol: u8,
}

impl SoundCnt {
    pub fn new() -> Self {
        Self {
            psg_master_volume_r: 0,
            psg_master_volume_l: 0,
            psg_enable_r: SoundEnableFlags::new(),
            psg_enable_l: SoundEnableFlags::new(),
            psg_volume: 0,
            dma_sound_a_vol: 0,
            dma_sound_b_vol: 0,
        }
    }

    pub fn read(&self, byte: u8) -> u8 {
        match byte {
            0 => self.psg_master_volume_l << 4 | self.psg_master_volume_r,
            1 => self.psg_enable_l.read() << 4 | self.psg_enable_r.read(),
            2 => self.dma_sound_b_vol << 3 | self.dma_sound_a_vol << 2 | (self.psg_volume),
            _ => unreachable!(),
        }
    }

    pub fn write<const BYTE: u8>(&mut self, value: u8) {
        match BYTE {
            0 => {
                self.psg_master_volume_r = value & 0x7;
                self.psg_master_volume_l = value >> 4 & 0x7;
            }
            1 => {
                self.psg_enable_r.write(value & 0xF);
                self.psg_enable_l.write(value >> 4);
            }
            2 => {
                self.psg_volume = value & 0x3;
                self.dma_sound_a_vol = value >> 2 & 0x1;
                self.dma_sound_b_vol = value >> 3 & 0x1;
            }
            _ => unreachable!(),
        }
    }
}

pub struct SoundBias {
    pub bias_level: u16,
    amplitude_res: u8,
}

impl SoundBias {
    pub fn new() -> Self {
        Self {
            bias_level: 0x200,
            amplitude_res: 0,
        }
    }

    pub fn read(&self, byte: u8) -> u8 {
        match byte {
            0 => self.bias_level as u8,
            1 => self.amplitude_res << 6 | (self.bias_level >> 8) as u8,
            _ => unreachable!(),
        }
    }

    pub fn write<const BYTE: u8>(&mut self, value: u8) {
        match BYTE {
            0 => self.bias_level = self.bias_level & !0xFF | value as u16 & !0x1,
            1 => {
                self.bias_level = self.bias_level & !0x300 | ((value as u16) & 0x3) << 8;
                self.amplitude_res = (value >> 6) & 0x3;
            }
            _ => unreachable!(),
        }
    }
}
