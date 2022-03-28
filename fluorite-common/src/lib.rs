use once_cell::sync::OnceCell;
use std::{
    cell::UnsafeCell,
    ops::{Deref, DerefMut},
};

pub struct EasyCell<T>(OnceCell<UnsafeCell<T>>);

impl<T> EasyCell<T> {
    pub const fn new() -> Self {
        Self(OnceCell::new())
    }

    pub fn init<F: Fn() -> T>(&self, f: F) {
        self.0.set(UnsafeCell::new(f()));
    }

    pub fn init_get<F: Fn() -> T>(&self, f: F) -> &mut T {
        self.init(f);
        self.get_mut()
    }

    #[inline]
    pub fn get(&self) -> &T {
        unsafe { &*self.0.get().unwrap_unchecked().get() }
    }

    #[inline]
    pub fn get_mut(&self) -> &mut T {
        unsafe { &mut *self.0.get().unwrap_unchecked().get() }
    }
}

impl<T> Deref for EasyCell<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.get()
    }
}

impl<T> DerefMut for EasyCell<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.get_mut()
    }
}

unsafe impl<T> Sync for EasyCell<T> {}
