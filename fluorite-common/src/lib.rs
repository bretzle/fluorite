#![doc(html_logo_url = "https://raw.githubusercontent.com/bretzle/fluorite/main/fluorite.png")]

mod bits;
mod buffer;
mod ptr;
mod shared;

pub use bits::BitIndex;
pub use buffer::*;
pub use ptr::WeakPointer;
pub use shared::Shared;

#[macro_export]
macro_rules! index2d {
    ($x:expr, $y:expr, $w:expr) => {
        $w * $y + $x
    };
    ($t:ty, $x:expr, $y:expr, $w:expr) => {
        (($w as $t) * ($y as $t) + ($x as $t)) as $t
    };
}
