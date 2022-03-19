use std::ops::{Deref, DerefMut};

#[repr(transparent)]
#[derive(Clone, Debug)]
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

impl<T> From<&mut T> for WeakPointer<T> {
    fn from(r: &mut T) -> Self {
        WeakPointer::new(r as *mut T)
    }
}