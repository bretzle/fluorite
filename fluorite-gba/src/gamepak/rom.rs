use std::{io, mem::size_of, path::Path};

const MAX_SIZE: usize = 32 * 1024 * 1024;

#[derive(Default)]
pub struct Rom {
    pub data: Vec<u8>,
    pub mask: usize,
    pub code: String,
    pub title: String,
}

impl Rom {
    pub fn load(&mut self, path: &Path) -> io::Result<()> {
        let mut data = std::fs::read(path)?;
        let size = data.len();

        unsafe { data.set_len(data.capacity()) };

        if size <= size_of::<Header>() || size > MAX_SIZE {
            panic!("Invalid ROM size: {size} bytes");
        }

        for addr in size..data.capacity() {
            self.data[addr] = (((addr >> 1) >> (8 * (addr & 0x1))) & 0xFF) as u8
        }

        let header = Header::new(&data);

        if header.fixed_96h == 0x96 && header.complement == header.calc_complement() {
            self.title = String::from_utf8(header.game_title.to_vec()).unwrap();
            self.code = String::from_utf8(header.game_code.to_vec()).unwrap();
        }

        Ok(())
    }

    pub fn read<T: Copy>(&self, addr: u32) -> T {
        unsafe { *(&self.data[addr as usize] as *const u8 as *const T) }
    }
}

#[repr(C)]
struct Header {
    entry_point: [u8; 4],
    nintendo_logo: [u8; 156],
    game_title: [u8; 12],
    game_code: [u8; 4],
    maker_code: [u8; 2],
    fixed_96h: u8,
    unit_code: u8,
    device_type: u8,
    reserved: [u8; 7],
    game_version: u8,
    complement: u8,
    checksum: [u8; 2],
}

impl Header {
    pub fn new(data: &[u8]) -> &Self {
        unsafe { &*(data.as_ptr() as *const Self) }
    }

    fn calc_complement(&self) -> u8 {
        let slice = unsafe {
            std::slice::from_raw_parts(self as *const Self as *const u8, size_of::<Self>())
        };

        let val = slice.iter().fold(0u8, |acc, x| acc.wrapping_add(*x)) + 0x19;
        let val = -(val as i8);
        val as u8
    }
}
