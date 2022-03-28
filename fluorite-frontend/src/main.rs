#![doc(html_logo_url = "https://raw.githubusercontent.com/bretzle/fluorite/main/fluorite.png")]

use application::{Application, State};
use counter::FrameCounter;
use fluorite_common::EasyCell;
use limiter::FrameRateLimiter;

mod application;
mod config;
mod counter;
mod limiter;
mod video_ctx;

static BIOS: &[u8] = include_bytes!("../../roms/gba_bios.bin");
static ROM: &[u8] = include_bytes!("../../roms/first-1.gba");

pub static LIMITER: EasyCell<FrameRateLimiter> = EasyCell::new();
pub static COUNTER: EasyCell<FrameCounter> = EasyCell::new();

fn main() -> color_eyre::Result<()> {
    simple_logger::init().unwrap();
    color_eyre::install()?;

    let mut app = Application::init();

    let limiter = LIMITER.init_get(FrameRateLimiter::new);
    let counter = COUNTER.init_get(FrameCounter::new);

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
