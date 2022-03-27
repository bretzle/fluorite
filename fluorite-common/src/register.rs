use num::NumCast;
use std::{mem::size_of, ops::BitAndAssign};

pub type RegisterR<T> = Register<false, true, T>;
pub type RegisterW<T> = Register<true, false, T>;
pub type RegisterRW<T> = Register<true, true, T>;

pub trait RegisterValue: NumCast + Copy + BitAndAssign {}

impl RegisterValue for u8 {}
impl RegisterValue for u16 {}
impl RegisterValue for u32 {}

pub struct Register<const READ: bool, const WRITE: bool, T> {
    raw: T,
    data: T,
    mask: T,
}

impl<const READ: bool, const WRITE: bool, T> Register<READ, WRITE, T>
where
    T: RegisterValue,
{
    pub fn new(mask: T) -> Self {
        Self {
            raw: unsafe { std::mem::zeroed() },
            data: unsafe { std::mem::zeroed() },
            mask,
        }
    }
}

impl<const WRITE: bool, T> Register<true, WRITE, T>
where
    T: RegisterValue,
{
    pub fn read(&self, index: usize) -> u8 {
        assert!(index <= size_of::<T>());

        let value = num::cast::<T, u32>(self.data).unwrap();
        num::cast(value >> (8 * index)).unwrap()
    }
}

impl<const READ: bool, T> Register<READ, true, T>
where
    T: RegisterValue,
{
    pub fn write(&mut self, index: usize, byte: u8) {
        assert!(index <= size_of::<T>());

        let rptr = &mut self.raw as *mut T as *mut u8;
        let dptr = &mut self.data as *mut T as *mut u8;

        unsafe {
            *rptr.add(index) = byte;
            *dptr.add(index) = byte;
        }

        self.data &= self.mask;
    }
}
