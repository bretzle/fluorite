#![doc(html_logo_url = "https://raw.githubusercontent.com/bretzle/fluorite/main/fluorite.png")]

use crate::arm::registers::Reg;
use crate::debug::TextureWindow;
use crate::display::Display;
use arm::registers::Registers;
use gba::Gba;
use glfw::{Key, Modifiers};
use imgui::*;
use std::collections::VecDeque;
use std::thread;
use utils::WeakPointer;

mod arm;
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

    // let (render_tx, render_rx) = flume::unbounded();
    let (keypad_tx, _keypad_rx) = flume::unbounded();

    let rom = match std::env::args().nth(1) {
        Some(path) => std::fs::read(path).unwrap(),
        None => ROM.to_vec(),
    };

    let (mut gba, debug_windows_spec_mutex) = Gba::new(BIOS.to_vec(), rom);
    let mut registers: WeakPointer<Registers> = WeakPointer::default();
    {
        let regs = unsafe { &mut *(&mut registers as *mut _) };
        let gba = unsafe { &mut *(&mut gba as *mut Gba) };

        thread::spawn(move || {
            *regs = WeakPointer::from(&mut gba.cpu.regs);

            loop {
                gba.emulate_frame()
            }
        });
    }

    // let mut pixels_lock = None;

    let mut imgui = Context::create();
    let mut display = Display::new(&mut imgui);
    let mut paused = false;

    let mut map_window = TextureWindow::new("BG Map");
    let mut tiles_window = TextureWindow::new("Tiles");
    let mut palettes_window = TextureWindow::new("Palettes");

    let map_labels = ["0", "1", "2", "3"];
    let tiles_block_labels = ["0", "1", "2", "3", "OBJ"];

    let debug_windows = VecDeque::new();

    while !display.should_close() {
        if !paused {
            // debug_windows = render_rx.recv().unwrap();
            // pixels_lock.replace(pixels_mutex.lock().unwrap());
        }

        // let pixels = pixels_lock.take().unwrap();
        let mut debug_spec = debug_windows_spec_mutex.lock().unwrap();
        let mut debug_copy = debug_windows.clone();

        display.render(
            gba.get_pixels(),
            &keypad_tx,
            &mut imgui,
            |ui, keys_pressed, modifers| {
                if paused {
                    Window::new("Paused")
                        .no_decoration()
                        .always_auto_resize(true)
                        .build(ui, || {
                            ui.text("Paused");
                        });
                }

                if debug_spec.reg_enable {
                    use Reg::*;
                    #[rustfmt::skip]
                    Window::new("Registers").resizable(false).build(ui, || {
						ui.text(format!("R0   0x{VAL:08X?}  {VAL:10?}", VAL = registers.get_reg(R0)));
						ui.text(format!("R1   0x{VAL:08X?}  {VAL:10?}", VAL = registers.get_reg(R1)));
						ui.text(format!("R2   0x{VAL:08X?}  {VAL:10?}", VAL = registers.get_reg(R2)));
						ui.text(format!("R3   0x{VAL:08X?}  {VAL:10?}", VAL = registers.get_reg(R3)));
						ui.text(format!("R4   0x{VAL:08X?}  {VAL:10?}", VAL = registers.get_reg(R4)));
						ui.text(format!("R5   0x{VAL:08X?}  {VAL:10?}", VAL = registers.get_reg(R5)));
						ui.text(format!("R6   0x{VAL:08X?}  {VAL:10?}", VAL = registers.get_reg(R6)));
						ui.text(format!("R7   0x{VAL:08X?}  {VAL:10?}", VAL = registers.get_reg(R7)));
						ui.text(format!("R8   0x{VAL:08X?}  {VAL:10?}", VAL = registers.get_reg(R8)));
						ui.text(format!("R9   0x{VAL:08X?}  {VAL:10?}", VAL = registers.get_reg(R9)));
						ui.text(format!("R10  0x{VAL:08X?}  {VAL:10?}", VAL = registers.get_reg(R10)));
						ui.text(format!("R11  0x{VAL:08X?}  {VAL:10?}", VAL = registers.get_reg(R11)));
						ui.text(format!("R12  0x{VAL:08X?}  {VAL:10?}", VAL = registers.get_reg(R12)));
						ui.text(format!("R13  0x{VAL:08X?}  {VAL:10?}", VAL = registers.get_reg(R13)));
						ui.text(format!("R14  0x{VAL:08X?}  {VAL:10?}", VAL = registers.get_reg(R14)));
						ui.text(format!("R15  0x{VAL:08X?}  {VAL:10?}", VAL = registers.get_reg(R15)));
						ui.text(if registers.get_t() {"THUMB"} else {"ARM"});
						ui.text(format!("M: {:?} N: {} Z: {} C: {} V {}", registers.get_mode(), registers.get_n()as u8, registers.get_z()as u8, registers.get_c()as u8, registers.get_v()as u8));
					});
                }

                if debug_spec.map_enable {
                    let (pixels, width, height) = debug_copy.pop_front().unwrap();
                    let bg_i = &mut debug_spec.map_spec.bg_i;
                    map_window.render(ui, &keys_pressed, pixels, width, height, || {
                        debug::control_combo_with_arrows(
                            ui,
                            &keys_pressed,
                            bg_i,
                            map_labels.len() - 1,
                        );
                        // ComboBox::new("BG").build_simple(
                        //     ui,
                        //     bg_i,
                        //     &[0usize, 1, 2, 3],
                        //     &(|i| std::borrow::Cow::from(map_labels[*i])),
                        // );
                    });
                }

                if debug_spec.tiles_enable {
                    let (pixels, width, height) = debug_copy.pop_front().unwrap();
                    let spec = &mut debug_spec.tiles_spec;
                    let (_palette, block, _bpp8) =
                        (&mut spec.palette, &mut spec.block, &mut spec.bpp8);
                    tiles_window.render(ui, &keys_pressed, pixels, width, height, || {
                        debug::control_combo_with_arrows(
                            ui,
                            &keys_pressed,
                            block,
                            tiles_block_labels.len() - 1,
                        );
                        // ComboBox::new("Block").build_simple(
                        //     ui,
                        //     block,
                        //     &[0, 1, 2, 3, 4],
                        //     &(|i| std::borrow::Cow::from(tiles_block_labels[*i])),
                        // );
                        // ui.checkbox("256 colors", bpp8);
                        // if !*bpp8 {
                        //     ui.input_int("Palette", palette).step(1).build();
                        //     *palette = if *palette > 15 {
                        //         15
                        //     } else if *palette < 0 {
                        //         0
                        //     } else {
                        //         *palette
                        //     };
                        // }
                    });
                }

                if debug_spec.palettes_enable {
                    let (pixels, width, height) = debug_copy.pop_front().unwrap();
                    palettes_window.render(ui, &keys_pressed, pixels, width, height, || {});
                }

                /*let mut mem_region_i = mem_region as usize;
                Window::new(im_str!("Memory Viewer"))
                .build(ui, || {
                    debug::control_combo_with_arrows(ui, &keys_pressed, &mut mem_region_i, 8);
                    ComboBox::new(im_str!("Memory Region")).build_simple(ui, &mut mem_region_i,
                        &[0, 1, 2, 3, 4, 5, 6, 7, 8],
                        &(|i| std::borrow::Cow::from(ImString::new(
                            VisibleMemoryRegion::from_index(*i).get_name()
                    ))));
                    mem_editor.build_without_window(&ui);
                });
                mem_region = VisibleMemoryRegion::from_index(mem_region_i);*/

                if modifers.contains(&Modifiers::Control) {
                    if paused {
                        return;
                    }
                    if keys_pressed.contains(&Key::M) {
                        debug_spec.map_enable = !debug_spec.map_enable
                    }
                    if keys_pressed.contains(&Key::T) {
                        debug_spec.tiles_enable = !debug_spec.tiles_enable
                    }
                    if keys_pressed.contains(&Key::P) {
                        debug_spec.palettes_enable = !debug_spec.palettes_enable
                    }
                } else if keys_pressed.contains(&Key::P) {
                    paused = !paused
                }
            },
        );

        drop(debug_spec);

        // if paused {
        //     pixels_lock = Some(pixels);
        // }
    }

    Ok(())
}
