use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, Instant},
};

use cpu::Gba;
use minifb::{Window, WindowOptions};

mod bios;
mod cartridge;
mod consts;
mod cpu;
mod dma;
mod gpu;
mod interrupt;
mod iodev;
mod sched;
mod sysbus;

struct MiniFb {
    window: minifb::Window,
}

pub trait VideoInterface {
    fn render(&mut self, buffer: &[u32]);
}

impl VideoInterface for MiniFb {
    fn render(&mut self, buffer: &[u32]) {
        self.window.update_with_buffer(buffer, 240, 160).unwrap();
    }
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    // let mut cpu = Gba::new();
    // cpu.skip_bios();

    // cpu.run();

    let fb = Rc::new(RefCell::new(MiniFb {
        window: Window::new(
            "fluorite",
            240,
            160,
            WindowOptions {
                borderless: true,
                scale: minifb::Scale::X4,
                ..Default::default()
            },
        )?,
    }));

    let mut gba = Gba::new(fb.clone());

    gba.skip_bios();

    let frame_time = Duration::new(0, 1_000_000_000 / 60);
    loop {
        let start_time = Instant::now();
        gba.frame();

        // TODO: update window title with fps

        // TODO: Add fps limiter
    }

    Ok(())
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
