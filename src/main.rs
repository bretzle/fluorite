#![allow(clippy::identity_op)]

use gba::Gba;
use raylib::prelude::*;
use std::fmt::Write;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
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

pub trait VideoInterface {
    fn render(&mut self, buffer: &[u32]);
}

static BIOS: &[u8] = include_bytes!("../roms/gba_bios.bin");

struct Screen {
    tex: RenderTexture2D,
    buffer: [u8; 4 * 240 * 160],
}

impl Screen {
    pub fn get_tex(&self) -> &RenderTexture2D {
        &self.tex
    }
}

impl VideoInterface for Screen {
    fn render(&mut self, buffer: &[u32]) {
        let now = Instant::now();
        for (idx, byte) in buffer.iter().enumerate() {
            let a = ((byte >> 24) & 0xFF) as u8;
            let r = ((byte >> 16) & 0xFF) as u8;
            let g = ((byte >> 8) & 0xFF) as u8;
            let b = ((byte >> 0) & 0xFF) as u8;
            self.buffer[idx * 4 + 0] = r;
            self.buffer[idx * 4 + 1] = g;
            self.buffer[idx * 4 + 2] = b;
            self.buffer[idx * 4 + 3] = 255 - a;
        }
        self.tex.update_texture(&self.buffer);
        println!("Texture upload took: {:?}", now.elapsed());
    }
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let (rom, name) = {
        let mut file_path = PathBuf::new();
        match std::env::args().nth(1) {
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

    let (mut rl, thread) = raylib::init()
        .size(240 * 4, 160 * 4)
        .title("Fluorite")
        .vsync()
        .build();

    rl.set_exit_key(None);

    println!("--------------");

    let tex = rl.load_render_texture(&thread, 240, 160).unwrap();

    let device = Rc::new(RefCell::new(Screen {
        tex,
        buffer: [0; 4 * 240 * 160],
    }));
    let mut counter = FpsCounter::default();
    let mut gba = Gba::new(device.clone(), BIOS, &rom);

    gba.skip_bios();
    let mut title = "".to_string();

    while !rl.window_should_close() {
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
            rl.set_window_title(&thread, &title);
        }

        let mut d = rl.begin_drawing(&thread);
        d.clear_background(Color::BLACK);
        // d.draw_texture(device.borrow().get_tex(), 100, 100, Color::WHITE);
        d.draw_texture_ex(
            device.borrow().get_tex(),
            Vector2::default(),
            0.0,
            4.0,
            Color::WHITE,
        );
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
