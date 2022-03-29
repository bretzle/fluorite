mod dma_sound;
mod noise;
mod tone;
mod wave;
mod components;

pub use components::Timer;
pub use dma_sound::DMASound;
pub use noise::Noise;
pub use tone::Tone;
pub use wave::Wave;

pub trait Channel {
    fn generate_sample(&self) -> i16;
    fn is_on(&self) -> bool;
}
