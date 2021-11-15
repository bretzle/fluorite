use gba::Gba;
use minifb::{Window, WindowOptions};
use std::fmt::Write;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, Instant},
};

mod bios;
mod cartridge;
mod consts;
mod dma;
mod gba;
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

static BIOS: &[u8] = include_bytes!("../roms/gba_bios.bin");

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let (rom, name) = {
        let mut file_path = PathBuf::new();
        match std::env::args().skip(1).next() {
            Some(s) => file_path.push(s),
            None => file_path.push("roms/beeg.bin"),
        };
        let mut file = File::open(&file_path)?;
        let mut buf = vec![];
        file.read_to_end(&mut buf)?;
        (
            buf,
            file_path.file_stem().unwrap().to_string_lossy().to_string(),
        )
    };

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
    let mut gba = Gba::new(fb.clone(), BIOS, &rom);

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
                "{} | fps: {} | Render: {} ({:?})",
                name,
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
