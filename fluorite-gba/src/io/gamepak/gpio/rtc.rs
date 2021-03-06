use chrono::{Datelike, Timelike};

use super::GpioDevice;
use crate::consts::CLOCK_FREQ;

pub struct Rtc {
    // Pins
    prev_sck: bool,
    sck: bool,
    sio: bool,
    cs: bool,
    // GPIO Registers
    is_used: bool,
    write_only: bool,
    write_mask: u8,
    // RTC Specific
    mode: Mode,
    last_byte: bool,
    counter: usize,
    date_time: DateTime,
}

impl Rtc {
    const IDENTIFIER_STRING: &'static [u8] = "SIIRTC_V".as_bytes();
    const COMMAND_CODE: u8 = 0b0110;
    const BIT_REVERSAL: [u8; 8] = [0, 4, 2, 6, 1, 5, 3, 7];

    pub fn new(rom: &[u8]) -> Self {
        let is_used = (0..(rom.len() - Rtc::IDENTIFIER_STRING.len()))
            .any(|i| rom[i..(i + Rtc::IDENTIFIER_STRING.len())] == *Rtc::IDENTIFIER_STRING)
            || std::env::args().any(|x| x == "--force-rtc");
        Self {
            // Pins
            prev_sck: false,
            sck: false,
            sio: false,
            cs: false,
            // GPIO Registers
            is_used,
            write_only: true,
            write_mask: 0b111,
            // RTC Specific
            mode: Mode::Start { done: false },
            counter: CLOCK_FREQ,
            last_byte: false,
            date_time: DateTime::new(),
        }
    }

    fn read_parameter(&mut self, parameter: Parameter) -> (u8, Parameter) {
        let value = match parameter {
            Parameter::Control(byte) => {
                self.last_byte = byte == 0;
                (self.date_time.control.read(), Parameter::Control(byte + 1))
            }
            Parameter::DateTime(byte) => {
                self.last_byte = byte == 6;
                (self.date_time.read(byte), Parameter::DateTime(byte + 1))
            }
            Parameter::Time(byte) => {
                self.last_byte = byte == 2;
                (self.date_time.read(byte + 4), Parameter::Time(byte + 1))
            }
            Parameter::Reset => {
                self.date_time = DateTime::new();
                self.last_byte = true;
                (0, Parameter::Reset)
            }
            Parameter::Irq => {
                todo!("RTC IRQ");
                // self.last_byte = true;
                // (0, Parameter::IRQ)
            }
        };
        value
    }

    fn write_parameter(&mut self, parameter: Parameter, value: u8) -> Parameter {
        match parameter {
            Parameter::Control(byte) => {
                self.date_time.control.write(value);
                self.last_byte = byte == 0;
                Parameter::Control(byte + 1)
            }
            Parameter::DateTime(byte) => {
                self.date_time.write(byte as u8, value);
                self.last_byte = byte == 6;
                Parameter::DateTime(byte + 1)
            }
            Parameter::Time(byte) => {
                self.date_time.write(byte as u8 + 4, value);
                self.last_byte = byte == 2;
                Parameter::Time(byte + 1)
            }
            Parameter::Reset => {
                self.date_time = DateTime::new();
                self.last_byte = false;
                Parameter::Reset
            }
            Parameter::Irq => {
                // TODO: RTC Interrupts
                self.last_byte = false;
                Parameter::Irq
            }
        }
    }
}

impl GpioDevice for Rtc {
    fn clock(&mut self) {
        if !self.is_used {
            return;
        }
        if self.counter == 0 {
            self.counter = CLOCK_FREQ;
            if self.date_time.second.inc()
                && self.date_time.minute.inc()
                && self.date_time.hour.inc()
            {
                self.date_time.day_of_week.inc();
                // TODO: Use actual number of days in month
                if self.date_time.day.inc_with_max(30) && self.date_time.month.inc() {
                    self.date_time.year.inc();
                }
            }
        } else {
            self.counter -= 1
        }
    }

    fn process_write(&mut self) {
        self.mode = match self.mode {
            Mode::Start { done: false } => {
                assert!(!self.cs && self.sck);
                Mode::Start { done: true }
            }
            Mode::Start { done: true } if self.cs && self.sck => Mode::Set(0, 0),
            Mode::Start { done: true } => self.mode,

            Mode::Set(command, 7) if self.prev_sck && !self.sck => {
                let command = command | (self.sio as u8) << 7;
                let command = if command & 0xF == Rtc::COMMAND_CODE {
                    command >> 4
                } else {
                    debug!("Interpreting MSB RTC Command");
                    assert_eq!(command >> 4, Rtc::COMMAND_CODE);
                    Rtc::BIT_REVERSAL[((command & 0xF) >> 1) as usize] | (command & 0x1) << 3
                };
                let parameter = Parameter::from(command & 0x7);
                let (parameter, access_type) = if command >> 3 != 0 {
                    let (parameter_byte, next_parameter) = self.read_parameter(parameter);
                    (next_parameter, AccessType::Read(parameter_byte, 0))
                } else {
                    (parameter, AccessType::Write(0, 0))
                };
                if parameter == Parameter::Reset || parameter == Parameter::Irq {
                    Mode::End
                } else {
                    Mode::Exec(parameter, access_type)
                }
            }
            Mode::Set(command, bit) if self.prev_sck && !self.sck => {
                assert!(self.cs);
                Mode::Set(command | (self.sio as u8) << bit, bit + 1)
            }
            Mode::Set(_command, _bit) => self.mode,

            Mode::Exec(parameter, AccessType::Read(byte, 7)) if self.prev_sck && !self.sck => {
                let done = self.last_byte;
                self.sio = byte & 0x1 != 0;
                if done {
                    Mode::End
                } else {
                    let (parameter_byte, next_parameter) = self.read_parameter(parameter);
                    Mode::Exec(next_parameter, AccessType::Read(parameter_byte, 0))
                }
            }
            Mode::Exec(parameter, AccessType::Read(byte, bit)) if self.prev_sck && !self.sck => {
                self.sio = byte & 0x1 != 0;
                Mode::Exec(parameter, AccessType::Read(byte >> 1, bit + 1))
            }
            Mode::Exec(_parameter, AccessType::Read(_byte, _bit)) => self.mode,

            Mode::Exec(parameter, AccessType::Write(byte, 7)) if self.prev_sck && !self.sck => {
                let done = self.last_byte;
                self.write_parameter(parameter, byte | (self.sio as u8) << 7);
                if done {
                    Mode::End
                } else {
                    Mode::Exec(parameter, AccessType::Write(byte + 1, 0))
                }
            }
            Mode::Exec(parameter, AccessType::Write(byte, bit)) if self.prev_sck && !self.sck => {
                Mode::Exec(
                    parameter,
                    AccessType::Write(byte | (self.sio as u8) << bit, bit + 1),
                )
            }
            Mode::Exec(_parameter, AccessType::Write(_byte, _bit)) => self.mode,

            Mode::End if !self.cs && self.sck => Mode::Start { done: false },
            Mode::End => Mode::End,
        };
    }

    // TODO: Switch implementation to GPIO when adding more GPIO
    fn read(&self, byte: u8) -> u8 {
        match byte {
            0 => {
                let mut val = 0;
                if !self.can_write(0) {
                    val |= self.sck as u8;
                }
                if !self.can_write(1) {
                    val |= (self.sio as u8) << 1;
                }
                if !self.can_write(2) {
                    val |= (self.cs as u8) << 2;
                }
                val
            }
            1 => 0,
            2 => self.write_mask(),
            3 => 0,
            4 => self.write_only() as u8,
            5 => 0,
            _ => unreachable!(),
        }
    }

    fn write(&mut self, byte: u8, value: u8) {
        match byte {
            0 => {
                if self.can_write(0) {
                    assert_eq!(self.write_mask & 0x1, 1);
                    self.prev_sck = self.sck;
                    self.sck = value & 0x1 != 0;
                }
                if self.can_write(1) {
                    assert_eq!(self.write_mask >> 1 & 0x1, 1);
                    self.sio = value >> 1 & 0x1 != 0;
                }
                if self.can_write(2) {
                    assert_eq!(self.write_mask >> 2 & 0x1, 1);
                    self.cs = value >> 2 & 0x1 != 0;
                }
                self.process_write();
            }
            1 => (),
            2 => self.set_write_mask(value & 0xF),
            3 => (),
            4 => self.set_write_only(value & 0x1 == 0),
            5 => (),
            _ => unreachable!(),
        }
    }

    // fn set_data0(&mut self, value: bool) {
    //     assert_eq!(self.write_mask & 0x1, 1);
    //     self.prev_sck = self.sck;
    //     self.sck = value;
    // }
    // fn set_data1(&mut self, value: bool) {
    //     assert_eq!(self.write_mask >> 1 & 0x1, 1);
    //     self.sio = value;
    // }
    // fn set_data2(&mut self, value: bool) {
    //     assert_eq!(self.write_mask >> 2 & 0x1, 1);
    //     self.cs = value;
    // }

    // fn data0(&self) -> bool {
    //     self.sck
    // }
    // fn data1(&self) -> bool {
    //     self.sio
    // }
    // fn data2(&self) -> bool {
    //     self.cs
    // }

    fn is_used(&self) -> bool {
        self.is_used
    }
    fn write_mask(&self) -> u8 {
        self.write_mask
    }
    fn can_write(&self, bit: u8) -> bool {
        self.write_mask >> bit & 0x1 != 0
    }
    fn set_write_mask(&mut self, value: u8) {
        self.write_mask = value
    }
    fn write_only(&self) -> bool {
        self.write_only
    }
    fn set_write_only(&mut self, value: bool) {
        self.write_only = value
    }

    // fn set_data3(&mut self, _value: bool) {
    //     assert_eq!(self.write_mask >> 3 & 0x1, 0)
    // }
    // fn data3(&self) -> bool {
    //     false
    // }
}

#[derive(Clone, Copy, Debug)]
enum Mode {
    Start { done: bool },
    Set(u8, usize),
    Exec(Parameter, AccessType),
    End,
}

#[derive(Clone, Copy, Debug)]
enum AccessType {
    Read(u8, usize),
    Write(u8, usize),
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Parameter {
    Control(u8),
    DateTime(u8),
    Time(u8),
    Reset,
    Irq,
}

impl Parameter {
    pub fn from(value: u8) -> Self {
        match value {
            4 => Parameter::Control(0),
            2 => Parameter::DateTime(0),
            6 => Parameter::Time(0),
            0 => Parameter::Reset,
            3 => Parameter::Irq,
            _ => panic!("Invalid RTC Command Parameter"),
        }
    }
}

struct Control {
    is_24h: bool,
    per_min_irq: bool,
}

impl Control {
    pub fn new() -> Control {
        Control {
            is_24h: false,
            per_min_irq: false,
        }
    }

    pub fn read(&self) -> u8 {
        (self.is_24h as u8) << 6 | (self.per_min_irq as u8) << 3
    }

    pub fn write(&mut self, value: u8) {
        self.is_24h = value >> 6 & 0x1 != 0;
        self.per_min_irq = value >> 3 & 0x1 != 0;
    }
}

struct DateTime {
    control: Control,
    // Date
    year: Bcd,
    month: Bcd,
    day: Bcd,
    day_of_week: Bcd,
    // Time
    is_pm: bool,
    hour: Bcd,
    minute: Bcd,
    second: Bcd,
}

impl DateTime {
    pub fn new() -> DateTime {
        let t = chrono::Local::now();

        fn bcd(val: u8) -> u8 {
            ((val / 10) << 4) | (val % 10)
        }

        let year = bcd((t.year() - 2000) as u8);
        let month = bcd(t.month() as u8);
        let day = bcd(t.day() as u8);
        let day_of_week = bcd(t.weekday().num_days_from_sunday() as u8);

        let (is_pm, hour) = t.hour12();
        let hour = bcd(hour as u8);
        let minute = bcd(t.minute() as u8);
        let second = bcd(t.second() as u8);

        DateTime {
            control: Control::new(),
            // Date
            year: Bcd::new(year, 0x99),
            month: Bcd::new(month, 0x12),
            day: Bcd::new(day, 0x30),
            day_of_week: Bcd::new(day_of_week, 0x07),
            // Time
            is_pm,
            hour: Bcd::new(hour, 0x23),
            minute: Bcd::new(minute, 0x59),
            second: Bcd::new(second, 0x59),
        }
    }

    fn read(&self, byte: u8) -> u8 {
        match byte {
            0 => self.year.value(),
            1 => self.month.value(),
            2 => self.day.value(),
            3 => self.day_of_week.value(),
            4 => {
                let hour = self.hour.value();
                let bit_6 = if self.control.is_24h {
                    hour >= 0x12
                } else {
                    self.is_pm
                };
                (bit_6 as u8) << 6 | hour
            }
            5 => self.minute.value(),
            6 => self.second.value(),
            _ => unreachable!(),
        }
    }

    fn write(&mut self, byte: u8, value: u8) {
        match byte {
            0 => self.year.set_value(value),
            1 => self.month.set_value(value),
            2 => self.day.set_value(value),
            3 => self.day_of_week.set_value(value),
            4 => {
                self.hour.set_value(value);
                if !self.control.is_24h {
                    self.is_pm = value >> 6 & 0x1 != 0
                };
            }
            5 => self.minute.set_value(value),
            6 => self.second.set_value(value),
            _ => unreachable!(),
        }
    }
}

struct Bcd {
    initial: u8,
    value: u8,
    max: u8,
}

impl Bcd {
    pub fn new(initial: u8, max: u8) -> Self {
        Self {
            initial,
            value: initial,
            max,
        }
    }

    pub fn inc(&mut self) -> bool {
        self.inc_with_max(self.max)
    }

    pub fn inc_with_max(&mut self, max: u8) -> bool {
        self.value += 1;
        if self.value > max {
            self.value = self.initial;
            assert!(self.value & 0xF < 0xA && self.value >> 4 < 0xA);
            true
        } else {
            if self.value & 0xF > 0x9 {
                // Shouldn't need to check overflow on upper nibble
                self.value = (self.value & 0xF0) + 0x10;
            }
            assert!(self.value & 0xF < 0xA && self.value >> 4 < 0xA);
            false
        }
    }

    pub fn value(&self) -> u8 {
        self.value
    }
    pub fn set_value(&mut self, value: u8) {
        self.value = value;
        assert!(self.value & 0xF < 0xA && self.value >> 4 < 0xA)
    }
}
