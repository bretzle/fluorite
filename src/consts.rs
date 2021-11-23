pub const GUI_PADDING: i32 = 10;
pub const GUI_ROW_HEIGHT: i32 = 30;
pub const GUI_LABEL_HEIGHT: i32 = 0;
pub const GUI_LABEL_PADDING: i32 = 5;
pub const GBA_LCD_W: f32 = 240.0;
pub const GBA_LCD_H: f32 = 160.0;

pub const _BANKED_REGS: [&str; 20] = [
    "SPSRfiq", "SPSRirq", "SPSRsvc", "SPSRabt", "SPSRund", "R8fiq", "R9fiq", "R10fiq", "R11fiq",
    "R12fiq", "R13fiq", "R14fiq", "R13irq", "R14irq", "R13svc", "R14svc", "R13abt", "R14abt",
    "R13und", "R14und",
];

#[repr(i32)]
#[derive(Debug, Clone, Copy)]
pub enum PanelMode {
    CPU = 0,
    IO = 1,
    AUDIO = 2,
}

pub const BUTTON_STATES: [PanelMode; 3] = [PanelMode::CPU, PanelMode::IO, PanelMode::AUDIO];

pub struct MmioRegBit {
    pub start: u8,
    pub size: u8,
    pub name: &'static str,
}

pub struct MmioReg {
    pub addr: u32,
    pub name: &'static str,
    pub bits: [MmioRegBit; 16],
}

#[rustfmt::skip]
pub const IO_REGS: [MmioReg; 2] = [
	MmioReg {
    	addr: 0x4000200,
    	name: "IE",
    	bits: [
			MmioRegBit { start: 0, size: 1, name: "LCD V-Blank" },
			MmioRegBit { start: 1, size: 1, name: "LCD H-Blank" },
			MmioRegBit { start: 2, size: 1, name: "LCD V-Counter Match" },
			MmioRegBit { start: 3, size: 1, name: "Timer 0 Overflow" },
			MmioRegBit { start: 4, size: 1, name: "Timer 0 Overflow" },
			MmioRegBit { start: 5, size: 1, name: "Timer 0 Overflow" },
			MmioRegBit { start: 6, size: 1, name: "Timer 0 Overflow" },
			MmioRegBit { start: 7, size: 1, name: "Serial Communication" },
			MmioRegBit { start: 8, size: 1, name: "DMA 0" },
			MmioRegBit { start: 9, size: 1, name: "DMA 1" },
			MmioRegBit { start: 10, size: 1, name: "DMA 2" },
			MmioRegBit { start: 11, size: 1, name: "DMA 3" },
			MmioRegBit { start: 12, size: 1, name: "Keypad" },
			MmioRegBit { start: 13, size: 1, name: "Game Pak (ext)" },
			MmioRegBit { start: 0, size: 0, name: "" },
			MmioRegBit { start: 0, size: 0, name: "" },
		],
	},
	MmioReg {
		addr: 0x04000004,
		name: "DISPSTAT",
		bits: [
			MmioRegBit { start: 0, size: 1, name: "V-Blank flag (1=VBlank) (set in line 160..226; not 227" },
			MmioRegBit { start: 1, size: 1, name: "H-Blank flag (1=HBlank) (toggled in all lines, 0..227" },
			MmioRegBit { start: 2, size: 1, name: "V-Counter flag (1=Match) (set in selected line)" },
			MmioRegBit { start: 3, size: 1, name: "V-Blank IRQ Enable (1=Enable)" },
			MmioRegBit { start: 4, size: 1, name: "H-Blank IRQ Enable (1=Enable)" },
			MmioRegBit { start: 5, size: 1, name: "V-Counter IRQ Enable (1=Enable)" },
			MmioRegBit { start: 6, size: 1, name: "DSi: LCD Initialization Ready (0=Busy, 1=Ready)" },
			MmioRegBit { start: 7, size: 1, name: "NDS: MSB of V-Vcount Setting (LYC.Bit8) (0..262)" },
			MmioRegBit { start: 8, size: 8, name: "V-Count Setting (LYC) (0..227)" },
			MmioRegBit { start: 0, size: 0, name: "" },
			MmioRegBit { start: 0, size: 0, name: "" },
			MmioRegBit { start: 0, size: 0, name: "" },
			MmioRegBit { start: 0, size: 0, name: "" },
			MmioRegBit { start: 0, size: 0, name: "" },
			MmioRegBit { start: 0, size: 0, name: "" },
			MmioRegBit { start: 0, size: 0, name: "" },
		]
	}
];
