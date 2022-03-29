use crate::LIMITER;

use super::{Application, State};

impl Application {
    pub(super) fn draw_imgui(&mut self) {
        let running = self.is_running();
        let is_fast_forward = LIMITER.is_fast_forward();

        self.video.draw(&self.events, |ui| {
            ui.main_menu_bar(|| {
                ui.menu("File", || {
                    if ui.menu_item_config("Open ROM").shortcut("Ctrl+O").build() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("roms", &["gba"])
                            .set_directory(&std::env::current_dir().unwrap())
                            .pick_file()
                        {
                            // TODO: update recent rom list

                            self.audio.pause();
                            self.gba.load_rom(path);
                            self.gba.reset();
                            self.state = State::Run;
                            self.audio.resume();
                            Application::queue_reset();
                        }
                    }

                    if ui.menu_item_config("Open save").enabled(running).build() {
                        todo!("Saves are not supported yet")
                    }

                    ui.menu_with_enabled("Recent", false, || todo!());

                    ui.separator();

                    ui.menu("Preferences", || {
                        if ui.menu_item("Settings") {
                            todo!()
                        }

                        if ui.menu_item("Keyboard config") {
                            todo!()
                        }

                        if ui.menu_item("Controller config") {
                            todo!()
                        }
                    });

                    ui.separator();

                    if ui.menu_item("Exit") {
                        self.state = State::Quit;
                    }
                });

                ui.menu("Emulation", || {
                    if ui
                        .menu_item_config("Reset")
                        .enabled(running)
                        .shortcut("Ctrl+R")
                        .build()
                    {
                        self.gba.reset();
                        Application::queue_reset();
                    }

                    if ui
                        .menu_item_config("Pause")
                        .shortcut("Ctrl+P")
                        .selected(self.state == State::Pause)
                        .enabled(running)
                        .build()
                    {
                        self.state = match self.state {
                            State::Run => {
                                self.audio.pause();
                                State::Pause
                            }
                            State::Pause => {
                                self.audio.resume();
                                State::Run
                            }
                            state => state,
                        }
                    }

                    ui.separator();

                    if ui
                        .menu_item_config("Fast forward")
                        .shortcut("Ctrl+Shift")
                        .selected(is_fast_forward)
                        .build()
                    {
                        LIMITER.get_mut().set_fast_forward(if is_fast_forward {
                            1.0
                        } else {
                            self.config.fast_forward as f64
                        });
                    }

                    ui.menu("Fast forward speed", || {
                        let multiplier = 1_000_000;

                        if ui
                            .menu_item_config("Unbound")
                            .shortcut("Ctrl+1")
                            .selected(self.config.fast_forward == multiplier)
                            .build()
                        {
                            todo!()
                        }

                        ui.separator();

                        for multiplier in 2..=8 {
                            if ui
                                .menu_item_config(&format!("{multiplier}x"))
                                .shortcut(&format!("Ctrl+{multiplier}"))
                                .selected(self.config.fast_forward == multiplier)
                                .build()
                            {
                                todo!()
                            }
                        }
                    });
                });

                ui.menu("Audio/Video", || {
                    ui.menu("Frame size", || {});

                    if ui
                        .menu_item_config("Preserve aspect ratio")
                        .selected(true)
                        .build()
                    {
                        todo!()
                    }

                    ui.separator();

                    ui.menu("Volume", || {
                        ui.slider("##", 0.0, 1.0, &mut self.config.volume);
                    });

                    if ui
                        .menu_item_config("Mute")
                        .shortcut("Ctrl+M")
                        .selected(self.config.mute)
                        .build()
                    {
                        self.config.mute ^= true;
                    }

                    ui.separator();

                    ui.menu("Video layers", || {});

                    ui.menu("Audio channels", || {});
                });

                ui.menu("Debug", || {
                    if ui
                        .menu_item_config("Registers")
                        .selected(self.show_registers)
                        .build()
                    {
                        self.show_registers ^= true;
                    }
                });
            });

            if self.show_registers {
                ui.window("Registers")
                    .opened(&mut self.show_registers)
                    .resizable(false)
                    .build(|| {
                        use fluorite_gba::arm::registers::Reg::*;
                        let regs = &self.gba.cpu.regs;
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
}
