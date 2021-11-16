use std::time::{Duration, Instant};

const SECOND: Duration = Duration::from_secs(1);

pub struct FpsCounter {
    count: u32,
    timer: Instant,
}

impl Default for FpsCounter {
    fn default() -> FpsCounter {
        FpsCounter {
            count: 0,
            timer: Instant::now(),
        }
    }
}

impl FpsCounter {
    pub fn tick(&mut self) -> Option<u32> {
        self.count += 1;
        if self.timer.elapsed() >= SECOND {
            let fps = self.count;
            self.timer = Instant::now();
            self.count = 0;
            Some(fps)
        } else {
            None
        }
    }
}
