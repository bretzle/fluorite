use crate::consts::{DISPLAY_HEIGHT, DISPLAY_WIDTH};

bitflags::bitflags! {
    #[derive(Default)]
    pub struct WindowFlags: u16 {
        const BG0 = 0b00000001;
        const BG1 = 0b00000010;
        const BG2 = 0b00000100;
        const BG3 = 0b00001000;
        const OBJ = 0b00010000;
        const SFX = 0b00100000;
    }
}

impl From<u16> for WindowFlags {
    fn from(v: u16) -> WindowFlags {
        WindowFlags::from_bits_truncate(v)
    }
}

impl WindowFlags {
    pub fn sfx_enabled(&self) -> bool {
        self.contains(WindowFlags::SFX)
    }
    pub fn bg_enabled(&self, bg: usize) -> bool {
        self.contains(BG_WIN_FLAG[bg])
    }
    pub fn obj_enabled(&self) -> bool {
        self.contains(WindowFlags::OBJ)
    }
}

const BG_WIN_FLAG: [WindowFlags; 4] = [
    WindowFlags::BG0,
    WindowFlags::BG1,
    WindowFlags::BG2,
    WindowFlags::BG3,
];

#[derive(Clone, Debug, Default)]
pub struct Window {
    pub left: u8,
    pub right: u8,
    pub top: u8,
    pub bottom: u8,
    pub flags: WindowFlags,
}

impl Window {
    pub fn inside(&self, x: usize, y: usize) -> bool {
        let left = self.left();
        let right = self.right();
        self.contains_y(y) && (x >= left && x < right)
    }

    pub fn left(&self) -> usize {
        self.left as usize
    }

    pub fn right(&self) -> usize {
        let left = self.left as usize;
        let mut right = self.right as usize;
        if right > DISPLAY_WIDTH || right < left {
            right = DISPLAY_WIDTH;
        }
        right
    }

    pub fn top(&self) -> usize {
        self.top as usize
    }

    pub fn bottom(&self) -> usize {
        let top = self.top as usize;
        let mut bottom = self.bottom as usize;
        if bottom > DISPLAY_HEIGHT || bottom < top {
            bottom = DISPLAY_HEIGHT;
        }
        bottom
    }

    pub fn contains_y(&self, y: usize) -> bool {
        let top = self.top();
        let bottom = self.bottom();
        y >= top && y < bottom
    }
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum WindowType {
    _Win0,
    _Win1,
    _WinObj,
    _WinOut,
    WinNone,
}

#[derive(Debug)]
pub struct WindowInfo {
    pub typ: WindowType,
    pub flags: WindowFlags,
}

impl WindowInfo {
    pub fn new(typ: WindowType, flags: WindowFlags) -> WindowInfo {
        WindowInfo { typ, flags }
    }

    pub fn _is_none(&self) -> bool {
        self.typ == WindowType::WinNone
    }
}
