#![doc(html_logo_url = "https://raw.githubusercontent.com/bretzle/fluorite/main/fluorite.png")]

use config::Config;
use counter::FrameCounter;
use fluorite_gba::{
    consts::{HEIGHT, WIDTH},
    gba::Gba,
};
use limiter::FrameRateLimiter;
use sdl2::{event::Event, EventPump, Sdl};
use video_ctx::VideoCtx;

mod config;
mod counter;
mod limiter;
mod video_ctx;

static BIOS: &[u8] = include_bytes!("../../roms/gba_bios.bin");
static ROM: &[u8] = include_bytes!("../../roms/first-1.gba");

fn main() -> color_eyre::Result<()> {
    simple_logger::init().unwrap();
    color_eyre::install()?;

    let mut app = Application::init();

    let mut limiter = FrameRateLimiter::new();
    let mut counter = FrameCounter::new();

    while app.state != State::Quit {
        limiter.run(|| {
            app.do_events();

            match app.state {
                State::Run | State::Pause => app.draw_frame(app.state),
                State::Menu => app.draw_menu(),
                _ => {}
            }
        });

        match app.state {
            State::Pause => counter.reset(),
            State::Menu => app.update_title(None),
            State::Run => {
                counter.inc();
                if let Some(fps) = counter.fps() {
                    app.update_title(Some(fps));
                }
            }
            _ => {}
        }
    }

    Ok(())
}

#[derive(PartialEq, Clone, Copy, Debug)]
enum State {
    Quit,
    Menu,
    Run,
    Pause,
}

struct Application {
    _sdl: Sdl,
    video: VideoCtx,
    _audio: (),
    _input: (),
    events: EventPump,
    config: Config,
    gba: Gba,
    state: State,

    show_ui: bool,
    show_registers: bool,
}

impl Application {
    pub fn init() -> Self {
        let sdl = sdl2::init().unwrap();
        let gba = Gba::new(BIOS.to_vec(), ROM.to_vec());
        Self {
            video: VideoCtx::init(&sdl),
            _audio: (),
            _input: (),
            events: sdl.event_pump().unwrap(),
            _sdl: sdl,
            config: Config::new(),
            gba,
            state: State::Menu,

            show_ui: true,
            show_registers: true,
        }
    }

    pub fn is_running(&self) -> bool {
        self.state == State::Run || self.state == State::Pause
    }

    pub fn update_title(&mut self, fps: Option<f64>) {
        self.video
            .window
            .set_title(&match fps {
                Some(_) => todo!(),
                None => format!("GBA Emulator"),
            })
            .unwrap()
    }

    pub fn do_events(&mut self) {
        for event in self.events.poll_iter() {
            self.video.handle_event(&event);

            match event {
                Event::Quit { .. } => self.state = State::Quit,
                _ => {}
            }
        }
    }

    pub fn draw_frame(&mut self, state: State) {
        if state == State::Run {
            const PIXELS_HOR: usize = WIDTH + 68;
            const PIXELS_VER: usize = HEIGHT + 68;
            const PIXEL_CUCLES: usize = 4;
            const FRAME_CYCLES: usize = PIXEL_CUCLES * PIXELS_HOR * PIXELS_VER;

            // keypad.update();
            // self.gba.run(kFrameCycles);
            todo!()
        } else {
            todo!()
            // video_ctx.renderFrame();
        }

        self.draw_menu();
    }

    pub fn draw_menu(&mut self) {
        self.draw_imgui();
        self.video.render(self.gba.get_pixels(), self.show_ui);
    }

    fn draw_imgui(&mut self) {
        if !self.show_ui {
            return;
        }

        let running = self.is_running();
        let is_fast_forward = false; // TODO: link this with the limiter

        self.video.draw(&self.events, |ui| {
            ui.main_menu_bar(|| {
                ui.menu("File", || {
                    if ui.menu_item_config("Open ROM").shortcut("Ctrl+O").build() {
                        // TODO: pause the audio
                        //       update recent rom list

                        let path = rfd::FileDialog::new()
                            .add_filter("roms", &["gba"])
                            .set_directory(&std::env::current_dir().unwrap())
                            .pick_file()
                            .unwrap();

                        self.gba.load_rom(path)
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
                        todo!()
                    }

                    if ui
                        .menu_item_config("Pause")
                        .shortcut("Ctrl+P")
                        .selected(self.state == State::Pause)
                        .enabled(running)
                        .build()
                    {
                        todo!()
                    }

                    ui.separator();

                    if ui
                        .menu_item_config("Fast forward")
                        .shortcut("Ctrl+Shift")
                        .selected(is_fast_forward)
                        .build()
                    {
                        todo!()
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
