use once_cell::sync::OnceCell;
use std::{
    cell::{Cell, UnsafeCell},
    lazy::SyncOnceCell,
    ops::{Deref, DerefMut},
};

/// This is incredibly unsafe. This is only for easy globals and should only ever be used on a single thread
pub struct EasyCell<T>(OnceCell<UnsafeCell<T>>);

impl<T> EasyCell<T> {
    pub const fn new() -> Self {
        Self(OnceCell::new())
    }

    pub fn init<F: Fn() -> T>(&self, f: F) {
        self.0
            .set(UnsafeCell::new(f()))
            .expect("Failed to init OnceCell");
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

///////////////////////////////////////////////////////////////////////////////

pub struct EasyLazy<T, F = fn() -> T> {
    cell: SyncOnceCell<T>,
    init: Cell<Option<F>>,
}

impl<T, F> EasyLazy<T, F> {
    pub const fn new(f: F) -> Self {
        Self {
            cell: SyncOnceCell::new(),
            init: Cell::new(Some(f)),
        }
    }
}

impl<T, F: FnOnce() -> T> Deref for EasyLazy<T, F> {
    type Target = T;

    fn deref(&self) -> &T {
        self.cell.get_or_init(|| match self.init.take() {
            Some(f) => f(),
            None => panic!("Lazy instance has previously been poisoned"),
        })
    }
}

impl<T, F: FnOnce() -> T> DerefMut for EasyLazy<T, F> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self.cell.get_mut() {
            Some(t) => t,
            None => panic!("Cannot get mutable reference"),
        }
    }
}

unsafe impl<T, F: Send> Sync for EasyLazy<T, F> {}
