use std::mem;
use std::ops::Range;

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

macro_rules! bit_index_impl {
	( $($ty:ty),* ) => {$(
		impl BitIndex for $ty {
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
	)*};
}


bit_index_impl!(u32, u16, u8);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bit() {
        assert_eq!(0x01u32.bit(0), true);
        assert_eq!(0x80u32.bit(7), true);
        assert_eq!(0xFEu32.bit(0), false);
        assert_eq!(0x7Fu32.bit(7), false);
    }

    #[test]
    #[should_panic]
    fn bit_panic() {
        0u32.bit(32);
    }

    #[test]
    fn set_bit() {
        assert_eq!(*0x01u32.set_bit(0, false), 0);
        assert_eq!(*0x80u32.set_bit(7, false), 0);
        assert_eq!(*0xFEu32.set_bit(0, true), 0xFF);
        assert_eq!(*0x7Fu32.set_bit(7, true), 0xFF);
        assert_eq!(*0x01u32.set_bit(0, true), 1);
        assert_eq!(*0x80u32.set_bit(7, true), 0x80);
        assert_eq!(*0xFEu32.set_bit(0, false), 0xFE);
        assert_eq!(*0x7Fu32.set_bit(7, false), 0x7F);
    }

    #[test]
    #[should_panic]
    fn set_bit_panic() {
        0u32.set_bit(33, false);
    }

    #[test]
    fn bit_range() {
        assert_eq!(0xAAu32.bit_range(0..3), 2);
        assert_eq!(0xAAu32.bit_range(4..8), 10);
    }

    #[test]
    #[should_panic]
    fn bit_range_panic() {
        0u32.bit_range(5..33);
    }

    #[test]
    fn set_bit_range() {
        assert_eq!(*0xAAu32.set_bit_range(0..3, 0b0110), 0xAE);
        assert_eq!(*0xAAu32.set_bit_range(4..8, 0b1100), 0xEA);
    }

    #[test]
    #[should_panic]
    fn set_bit_range_bounds_panic() {
        0u32.set_bit_range(5..33, 0);
    }

    #[test]
    #[should_panic]
    fn set_bit_range_value_length_panic() {
        0u32.set_bit_range(5..33, 0x1F);
    }
}
