use std::time::{Duration, Instant};

pub struct FrameRateLimiter {
    accumulated: Duration,
    frame_delta: Duration,
    fast_forward: f64,
    queue_reset: bool,
}

impl FrameRateLimiter {
    const REFRESH_RATE: f64 = 59.737;

    pub fn new() -> Self {
        let mut limiter = Self {
            accumulated: Duration::ZERO,
            frame_delta: Duration::ZERO,
            fast_forward: 1.0,
            queue_reset: false,
        };
        limiter.set_fps(Self::REFRESH_RATE);
        limiter
    }

    pub fn run<F: FnMut()>(&mut self, frame: F) {
        self.accumulated += self.measure(frame);

        if self.accumulated < self.frame_delta {
            self.accumulated += self.measure(|| {
                spin_sleep::sleep(self.frame_delta - self.accumulated);
            });
        }
        self.accumulated -= self.frame_delta;

        if self.queue_reset {
            self.reset();
        }
    }

    pub fn reset(&mut self) {
        self.accumulated = Duration::ZERO;
        self.queue_reset = false;
    }

    pub fn queue_reset(&mut self) {
        self.queue_reset = true;
    }

    pub fn is_fast_forward(&self) -> bool {
        self.fast_forward > 1.0
    }

    pub fn set_fast_forward(&mut self, fast_forward: f64) {
        self.fast_forward = fast_forward;
    }

    fn measure<F: FnMut()>(&self, mut callback: F) -> Duration {
        let begin = Instant::now();

        callback();

        begin.elapsed()
    }

    fn set_fps(&mut self, fps: f64) {
        self.frame_delta = Duration::from_secs_f64(1.0 / fps);
        self.queue_reset();
    }
}
