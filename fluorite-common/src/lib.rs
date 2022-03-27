pub extern crate num_traits as num;

mod ram;
mod register;
pub mod ptr;
pub mod cell;

pub use ram::Ram;
pub use register::{RegisterR, RegisterRW, RegisterW};

pub struct Event {
    when: u64,
    callback: fn(u64),
}

impl Event {
    #[must_use]
    pub fn new(callback: fn(u64)) -> Self {
        Self { when: 0, callback }
    }

    pub fn is_scheduled(&self) -> bool {
        self.when != 0
    }
}

impl PartialEq for Event {
    fn eq(&self, other: &Self) -> bool {
        self.when == other.when
    }
}

impl PartialOrd for Event {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.when.partial_cmp(&other.when)
    }
}
