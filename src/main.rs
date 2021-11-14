use gba::Gba;
use minifb::{Window, WindowOptions};
use std::fmt::Write;
use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, Instant},
};

mod bios;
mod cartridge;
mod consts;
mod gba;
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

    let mut counter = FpsCounter::default();
    let mut gba = Gba::new(fb.clone());

    gba.skip_bios();
    let mut title = "".to_string();

    loop {
        gba.frame();

        if let Some(real) = counter.tick() {
            let time = gba.render_time();
            let fps = 1.0 / time.as_secs_f64();
            title.clear();
            write!(
                &mut title,
                "yoshi_dma fps: {} | Render: {} ({:?})",
                real,
                fps.round(),
                time
            )?;

            let w = &mut fb.borrow_mut().window;
            w.set_title(&title);
            if !w.is_open() {
                break;
            }
        }
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

pub struct FpsCounter {
    count: u32,
    timer: Instant,
}

const SECOND: Duration = Duration::from_secs(1);

impl Default for FpsCounter {
    fn default() -> FpsCounter {
        FpsCounter {
            count: 0,
            timer: Instant::now(),
        }
    }
}

impl FpsCounter {
    pub fn tick(&mut self) -> Option<u32> {
        self.count += 1;
        if self.timer.elapsed() >= SECOND {
            let fps = self.count;
            self.timer = Instant::now();
            self.count = 0;
            Some(fps)
        } else {
            None
        }
    }
}
