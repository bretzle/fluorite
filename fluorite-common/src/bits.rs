pub trait BitRange<T> {
    fn bit_range<const START: usize, const END: usize>(self) -> T;
	#[must_use]
    fn set_bit_range<const START: usize, const END: usize>(self, value: T) -> Self;
}

macro_rules! impl_bitrange {
	((),($($bitrange_ty: ty),*)) => {};

    (($t: ty),($($bitrange_ty: ty),*)) => {
        $( impl_bitrange!($t, $bitrange_ty); )*
    };

    (($t_head: ty, $($t_rest: ty),*),($($bitrange_ty: ty),*)) => {
        impl_bitrange!(($t_head), ($($bitrange_ty),*));
        impl_bitrange!(($($t_rest),*), ($($bitrange_ty),*));
    };

    ($storage: ty, $value: ty) => {
        impl BitRange<$value> for $storage {
            #[inline]
            fn bit_range<const START: usize, const END: usize>(self) -> $value {
                const VALUE_BIT_LEN: usize = core::mem::size_of::<$value>() << 3;
                let selected = END - START;
                ((self >> START) as $value) << (VALUE_BIT_LEN - selected)
                    >> (VALUE_BIT_LEN - selected)
            }

            #[inline]
			#[must_use]
            fn set_bit_range<const START: usize, const END: usize>(self, value: $value) -> Self {
                const VALUE_BIT_LEN: usize = core::mem::size_of::<$value>() << 3;
                let selected = END - START;
                let mask = (if selected == VALUE_BIT_LEN {
                    <$value>::MAX
                } else {
                    ((1 as $value) << selected) - 1
                } as $storage)
                    << START;
                (self & !mask) | ((value as $storage) << START & mask)
            }
        }
    };
}

impl_bitrange!(
    (u8, u16, u32, u64, u128, i8, i16, i32, i64, i128),
    (u8, u16, u32, u64, u128, i8, i16, i32, i64, i128)
);

pub trait Bit {
    fn bit<const BIT: usize>(self) -> bool;
	#[must_use]
    fn set_bit<const BIT: usize>(self, value: bool) -> Self;
}

macro_rules! impl_bit {
	($($t:ty),*) => {
		$( impl_bit!(@ $t); )*
	};

	(@ $t:ty) => {
		impl Bit for $t {
			#[inline(always)]
			fn bit<const BIT: usize>(self) -> bool {
				self & 1 << BIT != 0
			}

			#[must_use]
			#[inline(always)]
			fn set_bit<const BIT: usize>(self, value: bool) -> Self {
				(self & !(1 << BIT)) | (value as $t) << BIT
			}
		}
	}
}

impl_bit!(u8, u16, u32, u64, u128, i8, i16, i32, i64, i128);
