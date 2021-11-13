pub const TIMING_VBLANK: u16 = 1;
pub const TIMING_HBLANK: u16 = 2;

pub trait DmaNotifier {
    fn notify(&mut self, timing: u16);
}
