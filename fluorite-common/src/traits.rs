pub trait UnsafeFrom<T> {
    unsafe fn from(_: T) -> Self;
}

impl<T, U> UnsafeFrom<U> for T
where
    T: From<U>,
{
    #[inline]
    unsafe fn from(other: U) -> Self {
        Self::from(other)
    }
}
