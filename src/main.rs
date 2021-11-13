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
mod gpu;
mod iodev;
mod sysbus;
mod sched;

struct MiniFb {
    window: minifb::Window,
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

    let mut gba = Gba::new();

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
