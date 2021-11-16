#![allow(clippy::identity_op)]

use fluorite_gba::gba::Gba;
use fluorite_gba::VideoInterface;
use raylib::prelude::*;
use std::fmt::Write;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use std::{cell::RefCell, rc::Rc};

use crate::fps::FpsCounter;

mod fps;

static BIOS: &[u8] = include_bytes!("../roms/gba_bios.bin");

struct Screen(RenderTexture2D);

impl Screen {
    pub fn get_tex(&self) -> &RenderTexture2D {
        &self.0
    }
}

impl VideoInterface for Screen {
    fn render(&mut self, buffer: &[u8]) {
        self.0.update_texture(buffer);
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

    let device = Rc::new(RefCell::new(Screen(tex)));
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
