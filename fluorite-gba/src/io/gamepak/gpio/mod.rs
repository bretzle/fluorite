use enum_dispatch::enum_dispatch;
use rtc::Rtc;

mod rtc;

#[enum_dispatch(Gpio)]
pub trait GpioDevice {
    fn clock(&mut self);
    fn process_write(&mut self);
    fn read(&self, byte: u8) -> u8;
    fn write(&mut self, byte: u8, value: u8);

    fn set_data0(&mut self, value: bool);
    fn set_data1(&mut self, value: bool);
    fn set_data2(&mut self, value: bool);
    fn set_data3(&mut self, value: bool);

    fn data0(&self) -> bool;
    fn data1(&self) -> bool;
    fn data2(&self) -> bool;
    fn data3(&self) -> bool;

    fn is_used(&self) -> bool;
    fn write_mask(&self) -> u8;
    fn can_write(&self, bit: u8) -> bool;
    fn set_write_mask(&mut self, value: u8);
    fn write_only(&self) -> bool;
    fn set_write_only(&mut self, value: bool);
}

#[enum_dispatch]
pub enum Gpio {
    Rtc,
}

impl Gpio {
    pub fn new(rom: &[u8]) -> Self {
        // TODO: dont assume that there is an RTC
        Rtc::new(rom).into()
    }

    pub fn read_register<D>(device: &D, offset: u32) -> u8
    where
        D: GpioDevice,
    {
        device.read(offset as u8)
    }

    pub fn write_register<D>(device: &mut D, offset: u32, val: u8)
    where
        D: GpioDevice,
    {
        device.write(offset as u8, val)
    }
}

impl Default for Gpio {
    fn default() -> Self {
        Self::Rtc(unsafe { std::mem::zeroed() })
    }
}
