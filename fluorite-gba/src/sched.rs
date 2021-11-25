use std::{cell::Cell, cmp::Ordering, collections::BinaryHeap};

const NUM_EVENTS: usize = 32;

#[derive(Debug, PartialOrd, PartialEq, Eq, Copy, Clone)]
pub enum EventType {
    RunLimitReached,
    Gpu(GpuEvent),
    Apu(ApuEvent),
    DmaActivateChannel(usize),
    TimerOverflow(usize),
}

#[derive(Debug, PartialOrd, PartialEq, Eq, Copy, Clone)]
pub enum GpuEvent {
    HDraw,
    HBlank,
    VBlankHDraw,
    VBlankHBlank,
}

#[derive(Debug, PartialOrd, PartialEq, Eq, Copy, Clone)]
pub enum ApuEvent {
    // TODO
}

#[derive(Debug, Clone, Eq)]
pub struct Event {
    ty: EventType,
    time: usize,
    cancel: Cell<bool>,
}

impl Event {
    fn new(ty: EventType, time: usize) -> Event {
        Event {
            ty,
            time,
            cancel: Cell::new(false),
        }
    }

    #[inline]
    fn get_type(&self) -> EventType {
        self.ty
    }

    fn is_canceled(&self) -> bool {
        self.cancel.get()
    }
}

impl Ord for Event {
    fn cmp(&self, other: &Self) -> Ordering {
        self.time.cmp(&other.time).reverse()
    }
}

/// Implement custom reverse ordering
impl PartialOrd for Event {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        other.time.partial_cmp(&self.time)
    }

    #[inline]
    fn lt(&self, other: &Self) -> bool {
        other.time < self.time
    }
    #[inline]
    fn le(&self, other: &Self) -> bool {
        other.time <= self.time
    }
    #[inline]
    fn gt(&self, other: &Self) -> bool {
        other.time > self.time
    }
    #[inline]
    fn ge(&self, other: &Self) -> bool {
        other.time >= self.time
    }
}

impl PartialEq for Event {
    fn eq(&self, other: &Self) -> bool {
        self.time == other.time
    }
}

#[derive(Debug, Clone)]
pub struct Scheduler {
    timestamp: usize,
    pub events: BinaryHeap<Event>,
}

impl Scheduler {
    pub fn new() -> Self {
        Self {
            timestamp: 0,
            events: BinaryHeap::with_capacity(NUM_EVENTS),
        }
    }

    pub fn timestamp(&self) -> usize {
        self.timestamp
    }

    pub fn timestamp_of_next_event(&self) -> usize {
        self.events.peek().unwrap_or_else(|| unreachable!()).time
    }

    pub fn push(&mut self, ty: EventType, cycles: usize) {
        let event = Event::new(ty, self.timestamp + cycles);
        self.events.push(event)
    }

    pub fn push_gpu_event(&mut self, e: GpuEvent, cycles: usize) {
        self.push(EventType::Gpu(e), cycles);
    }

    pub fn pop_pending_event(&mut self) -> Option<(EventType, usize)> {
        if let Some(event) = self.events.peek() {
            if self.timestamp >= event.time {
                // remove the event
                let event = self.events.pop().unwrap_or_else(|| unreachable!());
                if !event.is_canceled() {
                    Some((event.get_type(), self.timestamp - event.time))
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn fast_forward_to_next(&mut self) {
        self.timestamp += self.get_cycles_to_next_event();
    }

    pub fn get_cycles_to_next_event(&self) -> usize {
        if let Some(event) = self.events.peek() {
            event.time - self.timestamp
        } else {
            0
        }
    }

    pub fn update(&mut self, cycles: usize) {
        self.timestamp += cycles;
    }

    pub fn cancel(&mut self, ty: EventType) {
        self.events
            .iter()
            .filter(|e| e.ty == ty)
            .for_each(|e| e.cancel.set(true))
    }
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}
