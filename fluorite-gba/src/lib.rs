pub mod bios;
pub mod cartridge;
pub mod consts;
pub mod dma;
pub mod gba;
pub mod gpu;
pub mod interrupt;
pub mod iodev;
pub mod sched;
pub mod sysbus;

pub trait VideoInterface {
    fn render(&mut self, buffer: &[u8]);
}

#[macro_export]
macro_rules! index2d {
    ($x:expr, $y:expr, $w:expr) => {
        $w * $y + $x
    };
    ($t:ty, $x:expr, $y:expr, $w:expr) => {
        (($w as $t) * ($y as $t) + ($x as $t)) as $t
    };
}

pub trait GpuMemoryMappedIO {
    fn read(&self) -> u16;
    fn write(&mut self, value: u16);
}
