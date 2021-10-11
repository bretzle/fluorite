use std::cell::UnsafeCell;
use std::mem;
use std::ops::{Deref, DerefMut, Range};
use std::rc::Rc;

#[repr(transparent)]
#[derive(Debug)]
pub struct Shared<T>(Rc<UnsafeCell<T>>);

impl<T> Shared<T> {
    pub fn new(val: T) -> Self {
        Self(Rc::new(UnsafeCell::new(val)))
    }
}

impl<T> Deref for Shared<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.0.get() }
    }
}

impl<T> DerefMut for Shared<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.0.get() }
    }
}

impl<T> Clone for Shared<T> {
    fn clone(&self) -> Self {
        Self(Rc::clone(&self.0))
    }
}

impl<T: Default> Default for Shared<T> {
    fn default() -> Self {
        Shared::new(T::default())
    }
}

#[repr(transparent)]
#[derive(Debug)]
pub struct WeakPointer<T: ?Sized> {
    ptr: *mut T,
}

impl<T> WeakPointer<T> {
    pub fn new(ptr: *mut T) -> Self {
        WeakPointer { ptr }
    }
}

impl<T> Deref for WeakPointer<T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &(*self.ptr) }
    }
}

impl<T> DerefMut for WeakPointer<T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut (*self.ptr) }
    }
}

impl<T> Default for WeakPointer<T> {
    fn default() -> Self {
        WeakPointer {
            ptr: std::ptr::null_mut(),
        }
    }
}

/// A trait that provides methods for reading and setting bits / groups of bits
///
/// Note: None of the functions do any bounds checks
///
/// Note: position 0 is the least significant bit
pub trait BitIndex: Sized {
    /// Size of `Self` in bits
    const SIZE: usize = mem::size_of::<Self>() * 8;

    /// Obtains the value of the bit at the given position
    fn bit(&self, pos: usize) -> bool;

    /// Obtains the value of the bits inside the given range
    fn bit_range(&self, pos: Range<usize>) -> Self;

    /// Sets the value of the bit at the given position
    fn set_bit(&mut self, pos: usize, val: bool) -> &mut Self;

    /// Sets the value of the bits inside the given range
    fn set_bit_range(&mut self, pos: Range<usize>, val: Self) -> &mut Self;
}

impl BitIndex for u32 {
    fn bit(&self, pos: usize) -> bool {
        *self & 1 << pos != 0
    }

    fn bit_range(&self, pos: Range<usize>) -> Self {
        *self << Self::SIZE - pos.end >> Self::SIZE - pos.end + pos.start
    }

    fn set_bit(&mut self, pos: usize, val: bool) -> &mut Self {
        *self ^= (Self::MIN.wrapping_sub(val as Self) ^ *self) & 1 << pos;
        self
    }

    fn set_bit_range(&mut self, pos: Range<usize>, val: Self) -> &mut Self {
        let mask = !(Self::MIN.bit_range(pos.start..pos.end) << pos.start);
        *self = *self & mask | val << pos.start;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bit() {
        assert_eq!(0x01.bit(0), true);
        assert_eq!(0x80.bit(7), true);
        assert_eq!(0xFE.bit(0), false);
        assert_eq!(0x7F.bit(7), false);
    }

    #[test]
    #[should_panic]
    fn bit_panic() {
        0.bit(32);
    }

    #[test]
    fn set_bit() {
        assert_eq!(*0x01.set_bit(0, false), 0);
        assert_eq!(*0x80.set_bit(7, false), 0);
        assert_eq!(*0xFE.set_bit(0, true), 0xFF);
        assert_eq!(*0x7F.set_bit(7, true), 0xFF);
        assert_eq!(*0x01.set_bit(0, true), 1);
        assert_eq!(*0x80.set_bit(7, true), 0x80);
        assert_eq!(*0xFE.set_bit(0, false), 0xFE);
        assert_eq!(*0x7F.set_bit(7, false), 0x7F);
    }

    #[test]
    #[should_panic]
    fn set_bit_panic() {
        0.set_bit(33, false);
    }

    #[test]
    fn bit_range() {
        assert_eq!(0xAA.bit_range(0..3), 2);
        assert_eq!(0xAA.bit_range(4..8), 10);
    }

    #[test]
    #[should_panic]
    fn bit_range_panic() {
        0.bit_range(5..33);
    }

    #[test]
    fn set_bit_range() {
        assert_eq!(*0xAA.set_bit_range(0..3, 0b0110), 0xAE);
        assert_eq!(*0xAA.set_bit_range(4..8, 0b1100), 0xEA);
    }

    #[test]
    #[should_panic]
    fn set_bit_range_bounds_panic() {
        0.set_bit_range(5..33, 0);
    }

    #[test]
    #[should_panic]
    fn set_bit_range_value_length_panic() {
        0.set_bit_range(5..33, 0x1F);
    }
}