mod channel;
mod registers;

use channel::*;
use registers::*;

use crate::{consts::CLOCK_FREQ, gba::AUDIO_DEVICE};

use super::scheduler::Scheduler;

pub struct Apu {
    // Channels
    tone1: Tone,
    tone2: Tone,
    wave: Wave,
    noise: Noise,
    sound_a: DMASound,
    sound_b: DMASound,
    // Sound Control Registers
    cnt: SoundCnt,
    bias: SoundBias,
    master_enable: bool,

    // Sound Generation
    sample_clock: usize,
    fifo_a_req: bool,
    fifo_b_req: bool,
}

impl Apu {
    const CLOCKS_PER_SAMPLE: usize = CLOCK_FREQ / 0x8000;

    pub fn new() -> Self {
        Self {
            // Channels
            tone1: Tone::new(),
            tone2: Tone::new(),
            wave: Wave::new(),
            noise: Noise::new(),
            sound_a: DMASound::new(),
            sound_b: DMASound::new(),
            // Sound Control Registers
            cnt: SoundCnt::new(),
            bias: SoundBias::new(),
            master_enable: false,

            // Sound Generation
            sample_clock: Self::CLOCKS_PER_SAMPLE,
            fifo_a_req: false,
            fifo_b_req: false,
        }
    }

    pub fn clock(&mut self) {
        if !self.master_enable {
            return;
        }

        self.tone1.clock();
        self.tone2.clock();
        self.wave.clock();
        self.noise.clock();

        self.generate_sample();
    }

    pub fn on_timer_overflowed(&mut self, timer: usize) {
        self.fifo_a_req = self.sound_a.on_timer_overflowed(timer) || self.fifo_a_req;
        self.fifo_b_req = self.sound_b.on_timer_overflowed(timer) || self.fifo_b_req;
    }

    pub fn clock_sequencer(&mut self, step: usize) {
        match step {
            0 => self.clock_length_counters(),
            2 => {
                self.clock_length_counters();
                self.tone1.sweep.clock()
            }
            4 => self.clock_length_counters(),
            6 => {
                self.clock_length_counters();
                self.tone1.sweep.clock()
            }
            7 => self.clock_envelopes(),
            _ => assert!(step < 8),
        }
    }

    pub fn fifo_a_req(&mut self) -> bool {
        let fifo_a_req = self.fifo_a_req;
        self.fifo_a_req = false;
        fifo_a_req
    }

    pub fn fifo_b_req(&mut self) -> bool {
        let fifo_b_req = self.fifo_b_req;
        self.fifo_b_req = false;
        fifo_b_req
    }

    fn clock_length_counters(&mut self) {
        self.tone1.length_counter.clock();
        self.tone2.length_counter.clock();
        self.wave.length_counter.clock();
        self.noise.length_counter.clock();
    }

    fn clock_envelopes(&mut self) {
        self.tone1.envelope.clock();
        self.tone2.envelope.clock();
        self.noise.envelope.clock();
    }

    fn generate_sample(&mut self) {
        self.sample_clock -= 1;
        if self.sample_clock == 0 {
            let channel1_sample = self.tone1.generate_sample();
            let channel2_sample = self.tone2.generate_sample();
            let channel3_sample = self.wave.generate_sample();
            let channel4_sample = self.noise.generate_sample();
            let (mut psg_l, mut psg_r) = (0, 0);

            psg_l += self.cnt.psg_enable_l.channel1 as i16 * channel1_sample;
            psg_l += self.cnt.psg_enable_l.channel2 as i16 * channel2_sample;
            psg_l += self.cnt.psg_enable_l.channel3 as i16 * channel3_sample;
            psg_l += self.cnt.psg_enable_l.channel4 as i16 * channel4_sample;
            psg_r += self.cnt.psg_enable_r.channel1 as i16 * channel1_sample;
            psg_r += self.cnt.psg_enable_r.channel2 as i16 * channel2_sample;
            psg_r += self.cnt.psg_enable_r.channel3 as i16 * channel3_sample;
            psg_r += self.cnt.psg_enable_r.channel4 as i16 * channel4_sample;

            psg_l *= 1 + self.cnt.psg_master_volume_l as i16;
            psg_r *= 1 + self.cnt.psg_master_volume_r as i16;

            let sound_a_sample = DMASound::VOLUME_FACTORS[self.cnt.dma_sound_a_vol as usize]
                * self.sound_a.generate_sample();
            let sound_b_sample = DMASound::VOLUME_FACTORS[self.cnt.dma_sound_b_vol as usize]
                * self.sound_b.generate_sample();
            let (mut dma_l, mut dma_r) = (0, 0);

            dma_l += self.sound_a.enable_left as i16 * sound_a_sample;
            dma_l += self.sound_b.enable_left as i16 * sound_b_sample;
            dma_r += self.sound_a.enable_right as i16 * sound_a_sample;
            dma_r += self.sound_b.enable_right as i16 * sound_b_sample;

            let mut samples = [psg_l + dma_l, psg_r + dma_r];
            for sample in samples.iter_mut() {
                *sample = *sample + self.bias.bias_level as i16;
                *sample = num::clamp(*sample, 0, 0x3FF);
                *sample -= 0x200;
            }

            AUDIO_DEVICE.get_mut().write(samples);
            self.sample_clock = Self::CLOCKS_PER_SAMPLE;
        }
    }

    pub fn write_register(&mut self, _scheduler: &mut Scheduler, addr: u32, val: u8) {
        assert_eq!(addr >> 12, 0x04000);

        match addr & 0xFFF {
            0x082 => self.cnt.write(2, val),
            0x083 => {
                self.sound_a.write_cnt(val & 0xF);
                self.sound_b.write_cnt(val >> 4)
            }
            0x084 => {
                let prev = self.master_enable;
                self.master_enable = val >> 7 & 0x1 != 0;
                if !prev && self.master_enable {
                    self.tone1 = Tone::new();
                    self.tone2 = Tone::new();
                    self.wave = Wave::new();
                    self.noise = Noise::new();
                    self.cnt.write(0, val);
                    self.cnt.write(1, val);
                }
            }
            0x0A0..=0x0A3 => self.sound_a.write_fifo(val),
            0x0A4..=0x0A7 => self.sound_b.write_fifo(val),
            _ => panic!("Unimplemented APU Write 0x{addr:08X} = {val:02X}"),
        }
    }
}
