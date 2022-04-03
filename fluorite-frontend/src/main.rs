#![doc(html_logo_url = "https://raw.githubusercontent.com/bretzle/fluorite/main/fluorite.png")]
#![feature(once_cell)]
#![warn(clippy::pedantic)]
#![allow(
    clippy::cast_lossless,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::cast_possible_truncation,
    clippy::struct_excessive_bools,
    clippy::used_underscore_binding,
    clippy::too_many_lines,
    clippy::missing_panics_doc,
    clippy::cast_ptr_alignment,
    clippy::ptr_as_ptr,
    clippy::option_if_let_else,
    clippy::module_name_repetitions,
    clippy::verbose_bit_mask,
    clippy::wildcard_imports,
    clippy::must_use_candidate,
    clippy::unused_self,
    clippy::missing_errors_doc,
    clippy::if_same_then_else,
    clippy::new_without_default,
    clippy::enum_glob_use,
    clippy::unreadable_literal
)]

use application::{Application, State};
use counter::FrameCounter;
use fluorite_common::EasyCell;
use limiter::FrameRateLimiter;

mod application;
mod audio_ctx;
mod config;
mod counter;
mod limiter;
mod video_ctx;

pub static LIMITER: EasyCell<FrameRateLimiter> = EasyCell::new();
pub static COUNTER: EasyCell<FrameCounter> = EasyCell::new();

fn main() -> color_eyre::Result<()> {
    simple_logger::init().unwrap();
    color_eyre::install()?;

    let mut app = Application::new();

    app.init();

    let limiter = LIMITER.init_get(FrameRateLimiter::new);
    let counter = COUNTER.init_get(FrameCounter::new);

    while app.state != State::Quit {
        limiter.run(|| {
            app.do_events();

            match app.state {
                State::Run | State::Pause => app.draw_frame(app.state),
                State::Menu => app.draw_menu(),
                State::Quit => {}
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
            State::Quit => {}
        }
    }

    Ok(())
}
