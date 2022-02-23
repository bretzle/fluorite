#![doc(html_logo_url = "https://raw.githubusercontent.com/bretzle/fluorite/main/fluorite.png")]

use color_eyre::Result;
use fluorite_gba::gba::Gba;
use fluorite_gba::VideoInterface;
use raylib::audio::{AudioStream, RaylibAudio};
use raylib::texture::RaylibTexture2D;
use std::fmt::Write;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use std::{cell::RefCell, rc::Rc};

use crate::emu::EmulatorState;
use crate::fps::FpsCounter;

mod consts;
mod emu;
mod fps;
mod utils;

static BIOS: &[u8] = include_bytes!("../roms/gba_bios.bin");

fn read_rom(path: Option<String>, buffer: &mut Vec<u8>) -> Result<String> {
    let file_path = match path {
        None => {
            let mut file_path = PathBuf::new();
            match std::env::args().nth(1) {
                Some(s) => file_path.push(s),
                None => file_path.push("roms/beeg.gba"),
            };
            file_path
        }
        Some(s) => s.into(),
    };

    let mut file = File::open(&file_path)?;
    file.read_to_end(buffer)?;

    Ok(file_path.file_stem().unwrap().to_string_lossy().to_string())
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let (mut rl, thread) = raylib::init()
        .size(430 + (240 * 4), 160 * 4)
        .title("Fluorite")
        .vsync()
        .build();

    rl.set_exit_key(None);
    let ico = rl.load_texture(&thread, "fluorite.png").unwrap();
    rl.set_window_icon(ico.get_texture_data().unwrap());

    println!("--------------");

    let mut rom = vec![];
    let mut name = read_rom(None, &mut rom)?;

    let tex = rl.load_render_texture(&thread, 240, 160).unwrap();
    let emu = Rc::new(RefCell::new(EmulatorState::new(&thread, tex)));
    let mut counter = FpsCounter::default();
    let mut gba = Gba::new(emu.clone(), BIOS, &rom);

    gba.skip_bios();
    let mut title = String::with_capacity(32);

    while !rl.window_should_close() {
        {
            let mut emur = emu.borrow_mut();
            emur.poll_keys(&rl);
            match emur.run_state {
                0 => {
                    emur.reset();
                    emur.run_state = 1;
                    drop(emur);
                    gba = Gba::new(emu.clone(), BIOS, &rom);
                    gba.skip_bios();
                }
                1 => {}
                2 => {
                    // run
                    drop(emur);
                    gba.frame()
                }
                3 => {
                    // step
                    emur.run_state = 1;
                    drop(emur);
                    gba.run(1);
                }
                _ => unsafe { std::hint::unreachable_unchecked() },
            }
        }

        if rl.is_file_dropped() {
            if let Some(file_path) = rl.get_dropped_files().pop() {
                rl.clear_dropped_files();
                emu.borrow_mut().reset();
                rom.clear();
                name = read_rom(Some(file_path), &mut rom)?;
                gba = Gba::new(emu.clone(), BIOS, &rom);
                gba.skip_bios();
            }
        }

        let mut emu = emu.borrow_mut();
        emu.fps = rl.get_fps();

        if counter.tick().is_some() {
            let time = gba.render_time();
            let fps = 1.0 / time.as_secs_f64();
            title.clear();
            write!(
                &mut title,
                "{} | Render: {} ({:?})",
                name,
                fps.round(),
                time
            )?;
            rl.set_window_title(&thread, &title);
        }

        emu.draw_frame(&mut gba, &mut rl, &thread);
    }

    Ok(())
}
