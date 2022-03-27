use std::ops::{Deref, DerefMut};

// #[repr(transparent)]
#[derive(Debug)]
pub struct WeakPointer<T: ?Sized> {
    pub ptr: *mut T,
}

impl<T: ?Sized> Clone for WeakPointer<T> {
    fn clone(&self) -> Self {
        Self {
            ptr: self.ptr.clone(),
        }
    }
}

impl<T> WeakPointer<T> {
    pub fn new(ptr: *mut T) -> Self {
        Self { ptr }
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
        Self {
            ptr: std::ptr::null_mut(),
        }
    }
}
