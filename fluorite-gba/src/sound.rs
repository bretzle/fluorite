use std::{cell::RefCell, f32::consts::PI, rc::Rc};

use fluorite_arm::Addr;
use fluorite_common::{BitIndex, Shared};

use crate::{
    consts::*,
    sched::{ApuEvent, EventType, Scheduler},
    VideoInterface,
};

const DMG_RATIOS: [f32; 4] = [0.25, 0.5, 1.0, 0.0];
const DMA_TIMERS: [usize; 2] = [0, 1];
const DUTY_RATIOS: [f32; 4] = [0.125, 0.25, 0.5, 0.75];

const REG_FIFO_A_L: u32 = REG_FIFO_A;
const REG_FIFO_A_H: u32 = REG_FIFO_A + 2;

const REG_FIFO_B_L: u32 = REG_FIFO_B;
const REG_FIFO_B_H: u32 = REG_FIFO_B + 2;

const SOUND_FIFO_CAPACITY: usize = 32;

pub type StereoSample<T> = (T, T);

pub struct SoundController {
    scheduler: Shared<Scheduler>,

    _cycles: usize, // cycles count when we last provided a new sample.

    mse: bool,

    left_volume: usize,
    left_sqr1: bool,
    left_sqr2: bool,
    left_wave: bool,
    left_noise: bool,

    right_volume: usize,
    right_sqr1: bool,
    right_sqr2: bool,
    right_wave: bool,
    right_noise: bool,

    dmg_volume_ratio: f32,

    sqr1_rate: usize,
    sqr1_timed: bool,
    sqr1_length: f32,
    sqr1_duty: f32,
    sqr1_step_time: usize,
    sqr1_step_increase: bool,
    sqr1_initial_vol: usize,
    sqr1_cur_vol: usize,

    sound_bias: u16,

    sample_rate: f32,
    cycles_per_sample: usize,
    dma_sound: [DmaSoundChannel; 2],
    resampler: CosineResampler,
    output_buffer: Vec<StereoSample<f32>>,
}

impl SoundController {
    pub fn new(mut scheduler: Shared<Scheduler>, audio_device_sample_rate: f32) -> Self {
        let resampler = CosineResampler::new(32768_f32, audio_device_sample_rate);
        let cycles_per_sample = 512;
        scheduler.push(EventType::Apu(ApuEvent::Sample), cycles_per_sample);

        Self {
            scheduler,
            cycles_per_sample,
            _cycles: 0,
            mse: false,
            left_volume: 0,
            left_sqr1: false,
            left_sqr2: false,
            left_wave: false,
            left_noise: false,
            right_volume: 0,
            right_sqr1: false,
            right_sqr2: false,
            right_wave: false,
            right_noise: false,
            dmg_volume_ratio: 0.0,
            sqr1_rate: 0,
            sqr1_timed: false,
            sqr1_length: 0.0,
            sqr1_duty: DUTY_RATIOS[0],
            sqr1_step_time: 0,
            sqr1_step_increase: false,
            sqr1_initial_vol: 0,
            sqr1_cur_vol: 0,
            sound_bias: 0x200,
            sample_rate: 32_768f32,
            dma_sound: [Default::default(), Default::default()],
            resampler,
            output_buffer: Vec::with_capacity(1024),
        }
    }

    pub fn handle_read(&self, addr: Addr) -> u16 {
        let value = match addr {
            REG_SOUNDCNT_X => cbit(7, self.mse),
            REG_SOUNDCNT_L => {
                self.left_volume as u16
                    | (self.right_volume as u16) << 4
                    | cbit(8, self.left_sqr1)
                    | cbit(9, self.left_sqr2)
                    | cbit(10, self.left_wave)
                    | cbit(11, self.left_noise)
                    | cbit(12, self.right_sqr1)
                    | cbit(13, self.right_sqr2)
                    | cbit(14, self.right_wave)
                    | cbit(15, self.right_noise)
            }

            REG_SOUNDCNT_H => todo!(),

            REG_SOUNDBIAS => self.sound_bias,

            _ => {
                println!("Unimplemented read from {:x} {}", addr, io_reg_string(addr));
                0
            }
        };
        value
    }

    pub fn handle_write(&mut self, addr: Addr, val: u16) {
        if addr == REG_SOUNDCNT_X {
            if val & bit(7) != 0 {
                if !self.mse {
                    println!("MSE enabled!");
                    self.mse = true;
                }
            } else {
                if self.mse {
                    println!("MSE disabled!");
                    self.mse = false;
                }
            }

            // other fields of this register are read-only anyway, ignore them.
            return;
        }

        // TODO - figure out which writes should be disabled when MSE is off

        match addr {
            REG_SOUNDCNT_L => {
                self.left_volume = val.bit_range(0..2) as usize;
                self.right_volume = val.bit_range(4..6) as usize;
                self.left_sqr1 = val.bit(8);
                self.left_sqr2 = val.bit(9);
                self.left_wave = val.bit(10);
                self.left_noise = val.bit(11);
                self.right_sqr1 = val.bit(12);
                self.right_sqr2 = val.bit(13);
                self.right_wave = val.bit(14);
                self.right_noise = val.bit(15);
            }

            REG_SOUNDCNT_H => {
                self.dmg_volume_ratio = DMG_RATIOS[val.bit_range(0..1) as usize];
                self.dma_sound[0].volume_shift = val.bit(2) as i16;
                self.dma_sound[1].volume_shift = val.bit(3) as i16;
                self.dma_sound[0].enable_right = val.bit(8);
                self.dma_sound[0].enable_left = val.bit(9);
                self.dma_sound[0].timer_select = DMA_TIMERS[val.bit(10) as usize];
                self.dma_sound[1].enable_right = val.bit(12);
                self.dma_sound[1].enable_left = val.bit(13);
                self.dma_sound[1].timer_select = DMA_TIMERS[val.bit(14) as usize];

                if val.bit(11) {
                    self.dma_sound[0].fifo.reset();
                }
                if val.bit(15) {
                    self.dma_sound[1].fifo.reset();
                }
            }

            REG_SOUND1CNT_H => {
                self.sqr1_length = (64 - val.bit_range(0..5) as usize) as f32 / 256.0;
                self.sqr1_duty = DUTY_RATIOS[val.bit_range(6..7) as usize];
                self.sqr1_step_time = val.bit_range(8..10) as usize;
                self.sqr1_step_increase = val.bit(11);
                self.sqr1_initial_vol = val.bit_range(12..15) as usize;
            }

            REG_SOUND1CNT_X => {
                self.sqr1_rate = val.bit_range(0..10) as usize;
                self.sqr1_timed = val.bit(14);
                if val.bit(15) {
                    self.sqr1_cur_vol = self.sqr1_initial_vol;
                }
            }

            REG_FIFO_A_L | REG_FIFO_A_H => {
                self.dma_sound[0].fifo.write((val & 0xff) as i8);
                self.dma_sound[0].fifo.write(((val >> 8) & 0xff) as i8);
            }

            REG_FIFO_B_L | REG_FIFO_B_H => {
                self.dma_sound[1].fifo.write((val & 0xff) as i8);
                self.dma_sound[1].fifo.write(((val >> 8) & 0xff) as i8);
            }

            REG_SOUNDBIAS => {
                self.sound_bias = val & 0xc3fe;
                let resolution = self.sound_bias.bit_range(14..16) as usize;
                self.sample_rate = (32768 << resolution) as f32;
                if self.sample_rate != self.resampler.in_freq {
                    self.resampler.in_freq = self.sample_rate;
                }
                self.cycles_per_sample = 512 >> resolution;
                println!("bias - setting sample frequency to {}hz", self.sample_rate);
                // TODO this will not affect the currently scheduled sample event
            }

            _ => {
                // println!(
                //     "Unimplemented write to {:x} {}",
                //     io_addr,
                //     io_reg_string(io_addr)
                // );
            }
        }
    }

    pub fn on_event<T: VideoInterface>(
        &mut self,
        event: ApuEvent,
        extra_cycles: usize,
        device: &Rc<RefCell<T>>,
    ) {
        match event {
            ApuEvent::Sample => self.on_sample(extra_cycles, device),
            _ => println!("got {:?} event", event),
        }
    }

    fn on_sample<T: VideoInterface>(&mut self, extra_cycles: usize, audio_device: &Rc<RefCell<T>>) {
        let mut sample = [0f32; 2];

        for channel in 0..=1 {
            let mut dma_sample = 0;
            for dma in &mut self.dma_sound {
                if dma.is_stereo_channel_enabled(channel) {
                    let value = dma.value as i16;
                    dma_sample += value * (2 << dma.volume_shift);
                }
            }

            apply_bias(&mut dma_sample, self.sound_bias.bit_range(0..10) as i16);
            sample[channel] = dma_sample as i32 as f32;
        }

        let stereo_sample = (sample[0], sample[1]);
        self.resampler.feed(stereo_sample, &mut self.output_buffer);

        let mut audio = audio_device.borrow_mut();
        self.output_buffer.drain(..).for_each(|(left, right)| {
            audio.push_sample(&[
                (left.round() as i16) * (std::i16::MAX / 512),
                (right.round() as i16) * (std::i16::MAX / 512),
            ]);
        });

        self.scheduler
            .push_apu_event(ApuEvent::Sample, self.cycles_per_sample - extra_cycles);
    }
}

#[inline(always)]
fn apply_bias(sample: &mut i16, level: i16) {
    let mut s = *sample;
    s += level;
    // clamp
    if s > 0x3ff {
        s = 0x3ff;
    } else if s < 0 {
        s = 0;
    }
    s -= level;
    *sample = s;
}

fn cbit(idx: u8, value: bool) -> u16 {
    if value {
        1 << idx
    } else {
        0
    }
}

// TODO mvoe
fn bit(idx: u8) -> u16 {
    1 << idx
}

#[derive(Clone, Debug)]
struct DmaSoundChannel {
    value: i8,
    volume_shift: i16,
    enable_right: bool,
    enable_left: bool,
    timer_select: usize,
    fifo: SoundFifo,
}

impl DmaSoundChannel {
    fn is_stereo_channel_enabled(&self, channel: usize) -> bool {
        match channel {
            0 => self.enable_left,
            1 => self.enable_right,
            _ => unreachable!(),
        }
    }
}

impl Default for DmaSoundChannel {
    fn default() -> DmaSoundChannel {
        DmaSoundChannel {
            volume_shift: 0,
            value: 0,
            enable_right: false,
            enable_left: false,
            timer_select: 0,
            fifo: SoundFifo::new(),
        }
    }
}

// TODO write tests or replace with a crate

#[derive(Clone, Debug)]
pub struct SoundFifo {
    wr_pos: usize,
    rd_pos: usize,
    count: usize,
    data: [i8; SOUND_FIFO_CAPACITY],
}

impl SoundFifo {
    pub fn new() -> SoundFifo {
        SoundFifo {
            wr_pos: 0,
            rd_pos: 0,
            count: 0,
            data: [0; SOUND_FIFO_CAPACITY],
        }
    }

    pub fn write(&mut self, value: i8) {
        if self.count >= SOUND_FIFO_CAPACITY {
            return;
        }
        self.data[self.wr_pos] = value;
        self.wr_pos = (self.wr_pos + 1) % SOUND_FIFO_CAPACITY;
        self.count += 1;
    }

    pub fn read(&mut self) -> i8 {
        if self.count == 0 {
            return 0;
        };
        let value = self.data[self.rd_pos];
        self.rd_pos = (self.rd_pos + 1) % SOUND_FIFO_CAPACITY;
        self.count -= 1;
        value
    }

    pub fn count(&self) -> usize {
        self.count
    }

    pub fn reset(&mut self) {
        self.wr_pos = 0;
        self.rd_pos = 0;
        self.count = 0;
    }
}

pub trait Resampler {
    fn feed(&mut self, s: StereoSample<f32>, output: &mut Vec<StereoSample<f32>>);
}

#[derive(Clone, Debug)]
pub struct CosineResampler {
    last_in_sample: StereoSample<f32>,
    phase: f32,
    pub in_freq: f32,
    out_freq: f32,
}

fn cosine_interpolation(y1: f32, y2: f32, phase: f32) -> f32 {
    let mu2 = (1.0 - (PI * phase).cos()) / 2.0;
    y2 * (1.0 - mu2) + y1 * mu2
}

impl Resampler for CosineResampler {
    fn feed(&mut self, s: StereoSample<f32>, output: &mut Vec<StereoSample<f32>>) {
        while self.phase < 1.0 {
            let left = cosine_interpolation(self.last_in_sample.0, s.0, self.phase);
            let right = cosine_interpolation(self.last_in_sample.1, s.1, self.phase);
            output.push((left, right));
            self.phase += self.in_freq / self.out_freq;
        }
        self.phase = self.phase - 1.0;
        self.last_in_sample = s;
    }
}

impl CosineResampler {
    pub fn new(in_freq: f32, out_freq: f32) -> CosineResampler {
        CosineResampler {
            last_in_sample: Default::default(),
            phase: 0.0,
            in_freq: in_freq,
            out_freq: out_freq,
        }
    }
}
