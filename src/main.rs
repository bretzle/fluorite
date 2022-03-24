#![doc(html_logo_url = "https://raw.githubusercontent.com/bretzle/fluorite/main/fluorite.png")]

use crate::arm::registers::Reg;
use display::Display;
use gba::Gba;
use imgui::Context;
use std::{
    thread,
    time::{Duration, Instant},
};

mod arm;
// // mod debug;
mod display;
mod gba;
mod io;
mod utils;

static BIOS: &[u8] = include_bytes!("../roms/gba_bios.bin");
static ROM: &[u8] = include_bytes!("../roms/first-1.gba");

fn main() -> color_eyre::Result<()> {
    simple_logger::init().unwrap();
    color_eyre::install()?;

    let sdl = sdl2::init().unwrap();

    let mut imgui = Context::create();
    imgui.set_ini_filename(None);
    imgui.set_log_filename(None);

    // let (keypad_tx, _keypad_rx) = flume::unbounded();

    let rom = match std::env::args().nth(1) {
        Some(path) => std::fs::read(path).unwrap(),
        None => ROM.to_vec(),
    };

    let mut emu_fps = 60.0f32;
    let paused = false;

    let (mut gba, _debug_windows_spec_mutex) = Gba::new(BIOS.to_vec(), rom);
    {
        let gba = unsafe { &mut *(&mut gba as *mut Gba) };
        let emu_fps = unsafe { &mut *(&mut emu_fps as *mut f32) };
        let frame_lock = Duration::from_secs_f32(1.0 / 60.0);

        thread::spawn(move || loop {
            let before = Instant::now();
            gba.emulate_frame();
            let end = before.elapsed();

            if end < frame_lock {
                spin_sleep::sleep(frame_lock - end);
            }
            *emu_fps = 1.0 / before.elapsed().as_secs_f32();
        });
    }

    let mut display = Display::new(&sdl, &mut imgui);

    while !display.should_close() {
        display.render(gba.get_pixels(), emu_fps, &mut imgui, |ui| {
			if paused {
				ui.window( "Paused")
					.no_decoration()
					.always_auto_resize(true)
					.build(|| {
						ui.text("Paused");
					});
			}
			
			use Reg::*;
			let regs = &gba.cpu.regs;
			#[rustfmt::skip]
			ui.window("Registers").resizable(false).build( || {
				ui.text(format!("R0   0x{VAL:08X?}  {VAL:10?}", VAL = regs.get_reg(R0)));
				ui.text(format!("R1   0x{VAL:08X?}  {VAL:10?}", VAL = regs.get_reg(R1)));
				ui.text(format!("R2   0x{VAL:08X?}  {VAL:10?}", VAL = regs.get_reg(R2)));
				ui.text(format!("R3   0x{VAL:08X?}  {VAL:10?}", VAL = regs.get_reg(R3)));
				ui.text(format!("R4   0x{VAL:08X?}  {VAL:10?}", VAL = regs.get_reg(R4)));
				ui.text(format!("R5   0x{VAL:08X?}  {VAL:10?}", VAL = regs.get_reg(R5)));
				ui.text(format!("R6   0x{VAL:08X?}  {VAL:10?}", VAL = regs.get_reg(R6)));
				ui.text(format!("R7   0x{VAL:08X?}  {VAL:10?}", VAL = regs.get_reg(R7)));
				ui.text(format!("R8   0x{VAL:08X?}  {VAL:10?}", VAL = regs.get_reg(R8)));
				ui.text(format!("R9   0x{VAL:08X?}  {VAL:10?}", VAL = regs.get_reg(R9)));
				ui.text(format!("R10  0x{VAL:08X?}  {VAL:10?}", VAL = regs.get_reg(R10)));
				ui.text(format!("R11  0x{VAL:08X?}  {VAL:10?}", VAL = regs.get_reg(R11)));
				ui.text(format!("R12  0x{VAL:08X?}  {VAL:10?}", VAL = regs.get_reg(R12)));
				ui.text(format!("R13  0x{VAL:08X?}  {VAL:10?}", VAL = regs.get_reg(R13)));
				ui.text(format!("R14  0x{VAL:08X?}  {VAL:10?}", VAL = regs.get_reg(R14)));
				ui.text(format!("R15  0x{VAL:08X?}  {VAL:10?}", VAL = regs.get_reg(R15)));
				ui.text(if regs.get_t() {"THUMB"} else {"ARM"});
				ui.text(format!("M: {:?} N: {} Z: {} C: {} V {}", regs.get_mode(), regs.get_n()as u8, regs.get_z()as u8, regs.get_c()as u8, regs.get_v()as u8));
			});
		});
    }

    Ok(())
}
