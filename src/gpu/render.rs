use crate::consts::{DISPLAY_HEIGHT, DISPLAY_WIDTH};

pub type Point = (i32, i32);

#[derive(Debug)]
pub struct ViewPort {
    pub origin: Point,
    pub w: i32,
    pub h: i32,
}

impl ViewPort {
    pub fn new(w: i32, h: i32) -> Self {
        Self {
            origin: (0, 0),
            w,
            h,
        }
    }

    pub fn contains_point(&self, p: Point) -> bool {
        let (mut x, mut y) = p;

        x -= self.origin.0;
        y -= self.origin.1;

        x >= 0 && x < self.w && y >= 0 && y < self.h
    }
}

pub static SCREEN_VIEWPORT: ViewPort = ViewPort {
    origin: (0, 0),
    w: DISPLAY_WIDTH as i32,
    h: DISPLAY_HEIGHT as i32,
};
pub static MODE5_VIEWPORT: ViewPort = ViewPort {
    origin: (0, 0),
    w: 160,
    h: 128,
};

pub mod utils {
    use super::Point;

    #[inline]
    pub fn transform_bg_point(ref_point: Point, screen_x: i32, pa: i32, pc: i32) -> Point {
        let (ref_x, ref_y) = ref_point;
        ((ref_x + screen_x * pa) >> 8, (ref_y + screen_x * pc) >> 8)
    }
}
