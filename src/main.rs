#![doc(html_logo_url = "https://raw.githubusercontent.com/bretzle/fluorite/main/fluorite.png")]

use crate::arm::registers::Reg;
use debug::EmulatorState;
use display::Display;
use gba::Gba;
use imgui::Context;
use std::{
    thread,
    time::{Duration, Instant},
};

mod arm;
mod consts;
mod debug;
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
                // thread::sleep is not accurate enough on windows. Often sleeps can be off by ~16ms
                // spin_sleep is far more accurate by sleeping for a smaller acurate period of time
                // and then spinning for remaing duration
                spin_sleep::sleep(frame_lock - end);
            }
            *emu_fps = 1.0 / before.elapsed().as_secs_f32();
        });
    }

    let mut emu_state = EmulatorState::default();
    let mut display = Display::new(&sdl, &mut imgui);

    while !display.should_close() {
        display.render(gba.get_pixels(), emu_fps, &mut imgui, |ui| {
            if paused {
                ui.window("Paused")
                    .no_decoration()
                    .always_auto_resize(true)
                    .build(|| {
                        ui.text("Paused");
                    });
            }

            ui.main_menu_bar(|| {
                ui.menu("File", || {
                    if ui.menu_item("Open ROM") {
                        todo!("ROMs cant be loaded without restarting yet.")
                    }
                    if ui.menu_item("Load BIOS") {
                        todo!("Custom BIOS are not supported")
                    }
                    ui.separator();
                    if ui.menu_item("About") {
                        emu_state.show_about = true;
                    }
                    if ui.menu_item("Quit") {
                        std::process::exit(0)
                    }
                });

                ui.menu("Emulation", || {});

                ui.menu("Debug", || {});
            });

            if emu_state.show_about {
                ui.window("About")
                    .collapsible(false)
                    .resizable(false)
                    .opened(&mut emu_state.show_about)
                    .build(|| {
                        ui.text(env!("CARGO_PKG_NAME"));
                        ui.text(format!("version: {}", env!("CARGO_PKG_VERSION")));
                        ui.text(format!("branch: {}", consts::BRANCH));
                        ui.text(format!("revision: {}", consts::REVISION));
                    });
            }

            if emu_state.show_registers {
                ui.window("Registers")
                    .opened(&mut emu_state.show_registers)
                    .resizable(false)
                    .build(|| {
                        use Reg::*;
                        let regs = &gba.cpu.regs;
                        ui.text(format!("R0   0x{0:08X?}  {0:10?}", regs.get_reg(R0)));
                        ui.text(format!("R1   0x{0:08X?}  {0:10?}", regs.get_reg(R1)));
                        ui.text(format!("R2   0x{0:08X?}  {0:10?}", regs.get_reg(R2)));
                        ui.text(format!("R3   0x{0:08X?}  {0:10?}", regs.get_reg(R3)));
                        ui.text(format!("R4   0x{0:08X?}  {0:10?}", regs.get_reg(R4)));
                        ui.text(format!("R5   0x{0:08X?}  {0:10?}", regs.get_reg(R5)));
                        ui.text(format!("R6   0x{0:08X?}  {0:10?}", regs.get_reg(R6)));
                        ui.text(format!("R7   0x{0:08X?}  {0:10?}", regs.get_reg(R7)));
                        ui.text(format!("R8   0x{0:08X?}  {0:10?}", regs.get_reg(R8)));
                        ui.text(format!("R9   0x{0:08X?}  {0:10?}", regs.get_reg(R9)));
                        ui.text(format!("R10  0x{0:08X?}  {0:10?}", regs.get_reg(R10)));
                        ui.text(format!("R11  0x{0:08X?}  {0:10?}", regs.get_reg(R11)));
                        ui.text(format!("R12  0x{0:08X?}  {0:10?}", regs.get_reg(R12)));
                        ui.text(format!("R13  0x{0:08X?}  {0:10?}", regs.get_reg(R13)));
                        ui.text(format!("R14  0x{0:08X?}  {0:10?}", regs.get_reg(R14)));
                        ui.text(format!("R15  0x{0:08X?}  {0:10?}", regs.get_reg(R15)));
                        ui.text(format!("{}", regs.get_status()));
                    });
            }
        });
    }

    Ok(())
}
