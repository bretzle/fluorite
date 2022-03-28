use crate::video_ctx::VideoCtx;
use crate::ROM;
use crate::{config::Config, BIOS};
use fluorite_gba::{
    consts::{HEIGHT, WIDTH},
    gba::Gba,
};
use sdl2::{event::Event, EventPump, Sdl};

mod render;

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum State {
    Quit,
    Menu,
    Run,
    Pause,
}

pub struct Application {
    _sdl: Sdl,
    video: VideoCtx,
    _audio: (),
    _input: (),
    events: EventPump,
    config: Config,
    gba: Gba,
    pub state: State,

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
            state: State::Run,

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
                Some(fps) => format!("GBA Emulator - {fps:.1} FPS"),
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
            self.gba.run(FRAME_CYCLES);
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

    pub fn queue_reset() {
        crate::LIMITER.get_mut().queue_reset();
        crate::COUNTER.get_mut().queue_reset();
    }
}