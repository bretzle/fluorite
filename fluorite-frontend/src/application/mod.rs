use crate::audio_ctx::AudioCtx;
use crate::config::Config;
use crate::video_ctx::VideoCtx;
use fluorite_gba::{
    consts::{HEIGHT, WIDTH},
    gba::Gba,
};
use sdl2::{event::Event, EventPump, Sdl};

mod render;

#[derive(PartialEq, Clone, Copy)]
pub enum State {
    Quit,
    Menu,
    Run,
    Pause,
}

pub struct Application {
    _sdl: Sdl,
    video: VideoCtx,
    audio: AudioCtx,
    _input: (),
    events: EventPump,
    config: Config,
    gba: Gba,
    pub state: State,

    show_registers: bool,
}

impl Application {
    pub fn new() -> Self {
        let sdl = sdl2::init().unwrap();
        let gba = Gba::new();
        Self {
            video: VideoCtx::init(&sdl),
            audio: AudioCtx::new(&sdl),
            _input: (),
            events: sdl.event_pump().unwrap(),
            _sdl: sdl,
            config: Config::new(),
            gba,
            state: State::Menu,

            show_registers: true,
        }
    }

    pub fn init(&mut self) {
        self.audio.init();
        Gba::load_audio(&mut self.audio as *mut _);

        if let Some(path) = std::env::args().nth(1) {
            self.audio.pause();
            self.gba.load_rom(path);
            self.gba.reset();
            self.state = State::Run;
            self.audio.resume();
            Application::queue_reset();
        }
    }

    pub fn is_running(&self) -> bool {
        self.state == State::Run || self.state == State::Pause
    }

    pub fn update_title(&mut self, fps: Option<f64>) {
        self.video
            .window
            .set_title(&match fps {
                Some(fps) => format!("GBA Emulator - {fps:.1} FPS"),
                None => "GBA Emulator".to_string(),
            })
            .unwrap()
    }

    #[allow(clippy::single_match)]
    pub fn do_events(&mut self) {
        for event in self.events.poll_iter() {
            self.video.handle_event(&event);

            match event {
                Event::Quit { .. } => self.state = State::Quit,
                Event::DropFile { filename, .. } => {
                    // TODO: update recent rom list

                    self.audio.pause();
                    self.gba.load_rom(filename);
                    self.gba.reset();
                    self.state = State::Run;
                    self.audio.resume();
                    Application::queue_reset();
                }
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
            self.gba.run(FRAME_CYCLES);
        }

        self.draw_menu();
    }

    pub fn draw_menu(&mut self) {
        self.draw_imgui();
        self.video.render(self.gba.get_pixels());
    }

    pub fn queue_reset() {
        crate::LIMITER.get_mut().queue_reset();
        crate::COUNTER.get_mut().queue_reset();
    }
}
