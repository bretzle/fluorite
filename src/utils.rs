use crate::consts::{GUI_LABEL_HEIGHT, GUI_PADDING};
use raylib::{prelude::Rectangle, Raylib};
use std::ffi::CStr;

pub trait RectExt {
    /// Shave `padding` pixels off each side of the rectangle
    fn shave(&self, padding: i32) -> Rectangle;

    // Similar to `shave` but only on the left and right sides
    fn squeze(&self, amount: i32) -> Rectangle;

    /// Chop a `Rectangle` into to halves
    ///
    /// @return: (top, bottom)
    fn chop(&self, midpoint: i32, padding: i32) -> (Rectangle, Rectangle);
}

impl RectExt for Rectangle {
    fn shave(&self, padding: i32) -> Rectangle {
        let mut inside = *self;
        let padding = padding as f32;
        inside.x += padding;
        inside.y += padding;
        inside.width -= padding * 2.0;
        inside.height -= padding * 2.0;
        inside
    }

    fn squeze(&self, amount: i32) -> Rectangle {
        let mut ret = *self;
        let amount = amount as f32;
        ret.x += amount;
        ret.width -= amount * 2.0;
        ret
    }

    fn chop(&self, advance: i32, padding: i32) -> (Rectangle, Rectangle) {
        let mut top = *self;
        let mut bot = *self;

        top.height = advance as f32;
        bot.y += (advance + padding) as f32;
        bot.height -= (advance + padding) as f32;

        (top, bot)
    }
}

pub trait DrawExt {
    fn draw_label(&mut self, bounds: Rectangle, label: &str) -> Rectangle;
}

impl DrawExt for Raylib {
    fn draw_label(&mut self, layout_rect: Rectangle, label: &str) -> Rectangle {
        let (widget_rect, layout_rect) = layout_rect.chop(GUI_LABEL_HEIGHT, GUI_PADDING);
        self.GuiLabel(widget_rect, label);
        layout_rect
    }
}
