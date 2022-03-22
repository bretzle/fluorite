use crate::gba;
use priority_queue::PriorityQueue;
use std::cmp::Reverse;

pub struct Scheduler {
    pub cycle: usize,
    event_queue: PriorityQueue<EventType, Reverse<usize>>,
}

impl Scheduler {
    pub fn new() -> Self {
        let mut queue = PriorityQueue::new();
        queue.push(EventType::FrameSequencer(0), Reverse(gba::CLOCK_FREQ / 512));
        Self {
            cycle: 0,
            event_queue: queue,
        }
    }

    pub fn get_next_event(&mut self) -> Option<EventType> {
        // There should always be at least one event
        let (_event_type, cycle) = self.event_queue.peek().unwrap();
        if Reverse(self.cycle) == *cycle {
            Some(self.event_queue.pop().unwrap().0)
        } else {
            None
        }
    }

    pub fn add(&mut self, event: Event) {
        self.event_queue
            .push(event.event_type, Reverse(event.cycle));
    }

    pub fn _remove(&mut self, event_type: EventType) {
        self.event_queue.remove(&event_type);
    }
}

pub struct Event {
    pub cycle: usize,
    pub event_type: EventType,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EventType {
    _TimerOverflow(usize),
    FrameSequencer(usize),
}
