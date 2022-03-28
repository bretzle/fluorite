use once_cell::sync::OnceCell;
use std::cell::UnsafeCell;

pub struct EasyCell<T>(OnceCell<UnsafeCell<T>>);

impl<T> EasyCell<T> {
    pub const fn new() -> Self {
        Self(OnceCell::new())
    }

    pub fn init<F: FnOnce() -> T>(&self, f: F) -> &mut T {
        if let Err(_) = self.0.set(UnsafeCell::new(f())) {
            panic!("Failed to initialize OnceCell")
        }

        self.get_mut()
    }

    pub fn get(&self) -> &T {
        unsafe { &*self.0.get_unchecked().get() }
    }

    pub fn get_mut(&self) -> &mut T {
        unsafe { &mut *self.0.get_unchecked().get() }
    }
}

unsafe impl<T> Send for EasyCell<T> {}
unsafe impl<T> Sync for EasyCell<T> {}
