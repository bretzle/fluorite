use std::{marker::PhantomData, mem::size_of};

pub struct BasicMirror<const SIZE: usize>;

pub trait Mirror {
    fn mirror(addr: u32) -> u32;
}

impl<const SIZE: usize> Mirror for BasicMirror<SIZE> {
    fn mirror(addr: u32) -> u32 {
        addr & (SIZE - 1) as u32
    }
}

pub struct Ram<const SIZE: usize, M = BasicMirror<SIZE>> {
    pub data: Box<[u8; SIZE]>,
    _mirror: PhantomData<M>,
}

impl<const SIZE: usize, M> Default for Ram<SIZE, M> {
    fn default() -> Self {
        Self {
            data: Box::new([0; SIZE]),
            _mirror: Default::default(),
        }
    }
}

impl<const SIZE: usize, M> Ram<SIZE, M>
where
    M: Mirror,
{
    pub fn new(data: [u8; SIZE]) -> Self {
        Self {
            data: Box::new(data),
            _mirror: PhantomData,
        }
    }

    pub fn read_fast<T: Copy>(&self, addr: u32) -> T {
        assert!(addr as usize + size_of::<T>() <= SIZE);

        unsafe { *(&self.data[addr as usize] as *const u8 as *const T) }
    }

    pub fn write_fast<T: Copy>(&mut self, addr: u32, val: T) {
        assert!(addr as usize + size_of::<T>() <= SIZE);

        unsafe {
            *(&mut self.data[addr as usize] as *mut u8 as *mut T) = val;
        }
    }

    pub fn read_byte(&self, addr: u32) -> u8 {
        self.read(addr)
    }
    pub fn read_half(&self, addr: u32) -> u16 {
        self.read(addr)
    }
    pub fn read_word(&self, addr: u32) -> u32 {
        self.read(addr)
    }

    pub fn write_byte(&mut self, addr: u32, byte: u8) {
        self.write(addr, byte)
    }
    pub fn write_half(&mut self, addr: u32, half: u16) {
        self.write(addr, half)
    }
    pub fn write_word(&mut self, addr: u32, word: u32) {
        self.write(addr, word)
    }
}

impl<const SIZE: usize, M> Ram<SIZE, M>
where
    M: Mirror,
{
    fn read<T: Copy>(&self, addr: u32) -> T {
        let addr = addr & !(size_of::<T>() as u32 - 1);
        let addr = M::mirror(addr);

        self.read_fast(addr)
    }

    fn write<T: Copy>(&mut self, addr: u32, val: T) {
        let addr = addr & !(size_of::<T>() as u32 - 1);
        let addr = M::mirror(addr);

        self.write_fast(addr, val)
    }
}
