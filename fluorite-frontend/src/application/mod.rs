use crate::audio_ctx::AudioCtx;
use crate::video_ctx::VideoCtx;
use fluorite_common::flume::Sender;
use fluorite_gba::{
    consts::{HEIGHT, WIDTH},
    gba::Gba,
    io::keypad::KEYINPUT,
};
use sdl2::{event::Event, keyboard::Scancode, EventPump, Sdl};

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
    gba: Gba,
    pub state: State,
    key_tx: Sender<(u16, bool)>,
    show_registers: bool,
}

impl Application {
    pub fn new() -> Self {
        let sdl = sdl2::init().unwrap();
        let (tx, rx) = fluorite_common::flume::bounded(8);
        let gba = Gba::new(rx);
        Self {
            video: VideoCtx::init(&sdl),
            audio: AudioCtx::new(&sdl),
            _input: (),
            events: sdl.event_pump().unwrap(),
            _sdl: sdl,
            gba,
            state: State::Menu,
            key_tx: tx,
            show_registers: true,
        }
    }

    pub fn init(&mut self) {
        self.audio.init();
        Gba::load_audio(&mut self.audio);

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
            .unwrap();
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
                Event::KeyDown {
                    scancode: Some(code),
                    ..
                } => {
                    let key = match code {
                        Scancode::A => KEYINPUT::A,
                        Scancode::B => KEYINPUT::B,
                        Scancode::E => KEYINPUT::SELECT,
                        Scancode::T => KEYINPUT::START,
                        Scancode::Right => KEYINPUT::RIGHT,
                        Scancode::Left => KEYINPUT::LEFT,
                        Scancode::Up => KEYINPUT::UP,
                        Scancode::Down => KEYINPUT::DOWN,
                        Scancode::R => KEYINPUT::R,
                        Scancode::L => KEYINPUT::L,
                        _ => continue,
                    };
                    self.key_tx
                        .send((key, true))
                        .expect("Failed to send keypress");
                }
                Event::KeyUp {
                    scancode: Some(code),
                    ..
                } => {
                    let key = match code {
                        Scancode::A => KEYINPUT::A,
                        Scancode::B => KEYINPUT::B,
                        Scancode::E => KEYINPUT::SELECT,
                        Scancode::T => KEYINPUT::START,
                        Scancode::Right => KEYINPUT::RIGHT,
                        Scancode::Left => KEYINPUT::LEFT,
                        Scancode::Up => KEYINPUT::UP,
                        Scancode::Down => KEYINPUT::DOWN,
                        Scancode::R => KEYINPUT::R,
                        Scancode::L => KEYINPUT::L,
                        _ => continue,
                    };
                    self.key_tx
                        .send((key, false))
                        .expect("Failed to send keypress");
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
