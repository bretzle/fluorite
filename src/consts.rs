#![allow(unused_assignments)] // false positive in register macro

pub const GUI_PADDING: i32 = 10;
pub const GUI_ROW_HEIGHT: i32 = 30;
pub const GUI_LABEL_HEIGHT: i32 = 0;

pub const _BANKED_REGS: [&str; 20] = [
    "SPSRfiq", "SPSRirq", "SPSRsvc", "SPSRabt", "SPSRund", "R8fiq", "R9fiq", "R10fiq", "R11fiq",
    "R12fiq", "R13fiq", "R14fiq", "R13irq", "R14irq", "R13svc", "R14svc", "R13abt", "R14abt",
    "R13und", "R14und",
];

#[repr(i32)]
#[derive(Debug, Clone, Copy)]
pub enum PanelMode {
    Cpu = 0,
    Io = 1,
    Audio = 2,
}

pub const BUTTON_STATES: [PanelMode; 3] = [PanelMode::Cpu, PanelMode::Io, PanelMode::Audio];

#[derive(Copy, Clone)]
pub struct MmioRegBit {
    pub start: u8,
    pub size: u8,
    pub name: &'static str,
}

pub struct MmioReg {
    pub addr: u32,
    pub name: &'static str,
    pub bits: [Option<MmioRegBit>; 16],
}

macro_rules! register {
    ($addr:ident, $name:literal, $bits:tt) => {{
        let bits = register!(@BITS, $bits);
        MmioReg {
            addr: $addr,
            name: $name,
            bits,
        }
    }};

	(@BITS, [ $( {$start:literal, $size: literal, $name:literal}, )+ ]) => {{
		let mut bits = [None; 16];
		let mut idx = 0;

		$(
			bits[idx] = Some(
				MmioRegBit {
					start: $start,
					size: $size,
					name: $name,
				}
			);
			idx += 1;
		)*

		bits
	}};

	(@BITS, []) => {
		[None; 16]
	}
}

//////////////////////////////////////////////////////////////////////////////////////////
// MMIO Register listing from GBATEK (https://problemkaputt.de/gbatek.htm#gbamemorymap) //
//////////////////////////////////////////////////////////////////////////////////////////
// LCD MMIO Registers
const GBA_DISPCNT: u32 = 0x4000000; /* R/W LCD Control */
const GBA_GREENSWP: u32 = 0x4000002; /* R/W Undocumented - Green Swap */
const GBA_DISPSTAT: u32 = 0x4000004; /* R/W General LCD Status (STAT,LYC) */
const GBA_VCOUNT: u32 = 0x4000006; /* R   Vertical Counter (LY) */
const GBA_BG0CNT: u32 = 0x4000008; /* R/W BG0 Control */
const GBA_BG1CNT: u32 = 0x400000A; /* R/W BG1 Control */
const GBA_BG2CNT: u32 = 0x400000C; /* R/W BG2 Control */
const GBA_BG3CNT: u32 = 0x400000E; /* R/W BG3 Control */
const GBA_BG0HOFS: u32 = 0x4000010; /* W   BG0 X-Offset */
const GBA_BG0VOFS: u32 = 0x4000012; /* W   BG0 Y-Offset */
const GBA_BG1HOFS: u32 = 0x4000014; /* W   BG1 X-Offset */
const GBA_BG1VOFS: u32 = 0x4000016; /* W   BG1 Y-Offset */
const GBA_BG2HOFS: u32 = 0x4000018; /* W   BG2 X-Offset */
const GBA_BG2VOFS: u32 = 0x400001A; /* W   BG2 Y-Offset */
const GBA_BG3HOFS: u32 = 0x400001C; /* W   BG3 X-Offset */
const GBA_BG3VOFS: u32 = 0x400001E; /* W   BG3 Y-Offset */
const GBA_BG2PA: u32 = 0x4000020; /* W   BG2 Rotation/Scaling Parameter A (dx) */
const GBA_BG2PB: u32 = 0x4000022; /* W   BG2 Rotation/Scaling Parameter B (dmx) */
const GBA_BG2PC: u32 = 0x4000024; /* W   BG2 Rotation/Scaling Parameter C (dy) */
const GBA_BG2PD: u32 = 0x4000026; /* W   BG2 Rotation/Scaling Parameter D (dmy) */
const GBA_BG2X: u32 = 0x4000028; /* W   BG2 Reference Point X-Coordinate */
const GBA_BG2Y: u32 = 0x400002C; /* W   BG2 Reference Point Y-Coordinate */
const GBA_BG3PA: u32 = 0x4000030; /* W   BG3 Rotation/Scaling Parameter A (dx) */
const GBA_BG3PB: u32 = 0x4000032; /* W   BG3 Rotation/Scaling Parameter B (dmx) */
const GBA_BG3PC: u32 = 0x4000034; /* W   BG3 Rotation/Scaling Parameter C (dy) */
const GBA_BG3PD: u32 = 0x4000036; /* W   BG3 Rotation/Scaling Parameter D (dmy) */
const GBA_BG3X: u32 = 0x4000038; /* W   BG3 Reference Point X-Coordinate */
const GBA_BG3Y: u32 = 0x400003C; /* W   BG3 Reference Point Y-Coordinate */
const GBA_WIN0H: u32 = 0x4000040; /* W   Window 0 Horizontal Dimensions */
const GBA_WIN1H: u32 = 0x4000042; /* W   Window 1 Horizontal Dimensions */
const GBA_WIN0V: u32 = 0x4000044; /* W   Window 0 Vertical Dimensions */
const GBA_WIN1V: u32 = 0x4000046; /* W   Window 1 Vertical Dimensions */
const GBA_WININ: u32 = 0x4000048; /* R/W Inside of Window 0 and 1 */
const GBA_WINOUT: u32 = 0x400004A; /* R/W Inside of OBJ Window & Outside of Windows */
const GBA_MOSAIC: u32 = 0x400004C; /* W   Mosaic Size */
const GBA_BLDCNT: u32 = 0x4000050; /* R/W Color Special Effects Selection */
const GBA_BLDALPHA: u32 = 0x4000052; /* R/W Alpha Blending Coefficients */
const GBA_BLDY: u32 = 0x4000054; /* W   Brightness (Fade-In/Out) Coefficient */

// Sound Registers
const GBA_SOUND1CNT_L: u32 = 0x4000060; /* R/W   Channel 1 Sweep register       (NR10) */
const GBA_SOUND1CNT_H: u32 = 0x4000062; /* R/W   Channel 1 Duty/Length/Envelope (NR11, NR12) */
const GBA_SOUND1CNT_X: u32 = 0x4000064; /* R/W   Channel 1 Frequency/Control    (NR13, NR14) */
const GBA_SOUND2CNT_L: u32 = 0x4000068; /* R/W   Channel 2 Duty/Length/Envelope (NR21, NR22) */
const GBA_SOUND2CNT_H: u32 = 0x400006C; /* R/W   Channel 2 Frequency/Control    (NR23, NR24) */
const GBA_SOUND3CNT_L: u32 = 0x4000070; /* R/W   Channel 3 Stop/Wave RAM select (NR30) */
const GBA_SOUND3CNT_H: u32 = 0x4000072; /* R/W   Channel 3 Length/Volume        (NR31, NR32) */
const GBA_SOUND3CNT_X: u32 = 0x4000074; /* R/W   Channel 3 Frequency/Control    (NR33, NR34) */
const GBA_SOUND4CNT_L: u32 = 0x4000078; /* R/W   Channel 4 Length/Envelope      (NR41, NR42) */
const GBA_SOUND4CNT_H: u32 = 0x400007C; /* R/W   Channel 4 Frequency/Control    (NR43, NR44) */
const GBA_SOUNDCNT_L: u32 = 0x4000080; /* R/W   Control Stereo/Volume/Enable   (NR50, NR51) */
const GBA_SOUNDCNT_H: u32 = 0x4000082; /* R/W   Control Mixing/DMA Control */
const GBA_SOUNDCNT_X: u32 = 0x4000084; /* R/W   Control Sound on/off           (NR52) */
const GBA_SOUNDBIAS: u32 = 0x4000088; /* BIOS  Sound PWM Control */
const GBA_WAVE_RAM: u32 = 0x4000090; /* R/W Channel 3 Wave Pattern RAM (2 banks!!) */
const GBA_FIFO_A: u32 = 0x40000A0; /* W   Channel A FIFO, Data 0-3 */
const GBA_FIFO_B: u32 = 0x40000A4; /* W   Channel B FIFO, Data 0-3 */

// DMA Transfer Channels
const GBA_DMA0SAD: u32 = 0x40000B0; /* W    DMA 0 Source Address */
const GBA_DMA0DAD: u32 = 0x40000B4; /* W    DMA 0 Destination Address */
const GBA_DMA0CNT_L: u32 = 0x40000B8; /* W    DMA 0 Word Count */
const GBA_DMA0CNT_H: u32 = 0x40000BA; /* R/W  DMA 0 Control */
const GBA_DMA1SAD: u32 = 0x40000BC; /* W    DMA 1 Source Address */
const GBA_DMA1DAD: u32 = 0x40000C0; /* W    DMA 1 Destination Address */
const GBA_DMA1CNT_L: u32 = 0x40000C4; /* W    DMA 1 Word Count */
const GBA_DMA1CNT_H: u32 = 0x40000C6; /* R/W  DMA 1 Control */
const GBA_DMA2SAD: u32 = 0x40000C8; /* W    DMA 2 Source Address */
const GBA_DMA2DAD: u32 = 0x40000CC; /* W    DMA 2 Destination Address */
const GBA_DMA2CNT_L: u32 = 0x40000D0; /* W    DMA 2 Word Count */
const GBA_DMA2CNT_H: u32 = 0x40000D2; /* R/W  DMA 2 Control */
const GBA_DMA3SAD: u32 = 0x40000D4; /* W    DMA 3 Source Address */
const GBA_DMA3DAD: u32 = 0x40000D8; /* W    DMA 3 Destination Address */
const GBA_DMA3CNT_L: u32 = 0x40000DC; /* W    DMA 3 Word Count */
const GBA_DMA3CNT_H: u32 = 0x40000DE; /* R/W  DMA 3 Control */

// Timer Registers
const GBA_TM0CNT_L: u32 = 0x4000100; /* R/W   Timer 0 Counter/Reload */
const GBA_TM0CNT_H: u32 = 0x4000102; /* R/W   Timer 0 Control */
const GBA_TM1CNT_L: u32 = 0x4000104; /* R/W   Timer 1 Counter/Reload */
const GBA_TM1CNT_H: u32 = 0x4000106; /* R/W   Timer 1 Control */
const GBA_TM2CNT_L: u32 = 0x4000108; /* R/W   Timer 2 Counter/Reload */
const GBA_TM2CNT_H: u32 = 0x400010A; /* R/W   Timer 2 Control */
const GBA_TM3CNT_L: u32 = 0x400010C; /* R/W   Timer 3 Counter/Reload */
const GBA_TM3CNT_H: u32 = 0x400010E; /* R/W   Timer 3 Control */

// Serial Communication (1)
const GBA_SIODATA32: u32 = 0x4000120; /*R/W   SIO Data (Normal-32bit Mode; shared with below) */
const GBA_SIOMULTI0: u32 = 0x4000120; /*R/W   SIO Data 0 (Parent)    (Multi-Player Mode) */
const GBA_SIOMULTI1: u32 = 0x4000122; /*R/W   SIO Data 1 (1st Child) (Multi-Player Mode) */
const GBA_SIOMULTI2: u32 = 0x4000124; /*R/W   SIO Data 2 (2nd Child) (Multi-Player Mode) */
const GBA_SIOMULTI3: u32 = 0x4000126; /*R/W   SIO Data 3 (3rd Child) (Multi-Player Mode) */
const GBA_SIOCNT: u32 = 0x4000128; /*R/W   SIO Control Register */
const GBA_SIOMLT_SEND: u32 = 0x400012A; /*R/W   SIO Data (Local of MultiPlayer; shared below) */
const GBA_SIODATA8: u32 = 0x400012A; /*R/W   SIO Data (Normal-8bit and UART Mode) */

// Keypad Input
const GBA_KEYINPUT: u32 = 0x4000130; /* R      Key Status */
const GBA_KEYCNT: u32 = 0x4000132; /* R/W    Key Interrupt Control */

// Serial Communication (2)
const GBA_RCNT: u32 = 0x4000134; /* R/W  SIO Mode Select/General Purpose Data */
const GBA_IR: u32 = 0x4000136; /* -    Ancient - Infrared Register (Prototypes only) */
const GBA_JOYCNT: u32 = 0x4000140; /* R/W  SIO JOY Bus Control */
const GBA_JOY_RECV: u32 = 0x4000150; /* R/W  SIO JOY Bus Receive Data */
const GBA_JOY_TRANS: u32 = 0x4000154; /* R/W  SIO JOY Bus Transmit Data */
const GBA_JOYSTAT: u32 = 0x4000158; /* R/?  SIO JOY Bus Receive Status */

// Interrupt, Waitstate, and Power-Down Control
const GBA_IE: u32 = 0x4000200; /* R/W  IE        Interrupt Enable Register */
const GBA_IF: u32 = 0x4000202; /* R/W  IF        Interrupt Request Flags / IRQ Acknowledge */
const GBA_WAITCNT: u32 = 0x4000204; /* R/W  WAITCNT   Game Pak Waitstate Control */
const GBA_IME: u32 = 0x4000208; /* R/W  IME       Interrupt Master Enable Register */
const GBA_POSTFLG: u32 = 0x4000300; /* R/W  POSTFLG   Undocumented - Post Boot Flag */
const GBA_HALTCNT: u32 = 0x4000301; /* W    HALTCNT   Undocumented - Power Down Control */
// const GBA_?       :u32 =0x4000410;      /* ?    ?         Undocumented - Purpose Unknown / Bug ??? 0FFh */
// const GBA_?       :u32 =0x4000800;      /* R/W  ?         Undocumented - Internal Memory Control (R/W) */
// const GBA_?       :u32 =0x4xx0800;      /* R/W  ?         Mirrors of 4000800h (repeated each 64K) */
// const GBA_(3DS)   :u32 =0x4700000;      /* W    (3DS)     Disable ARM7 bootrom overlay (3DS only) */
pub const IO_REGS: &[MmioReg] = &[
    // Interrupt, Waitstate, and Power-Down Control
    register! {
        GBA_IE, "IE",
        [
            { 0 , 1, "LCD V-Blank" },
            { 1 , 1, "LCD H-Blank" },
            { 2 , 1, "LCD V-Counter Match" },
            { 3 , 1, "Timer 0 Overflow" },
            { 4 , 1, "Timer 1 Overflow" },
            { 5 , 1, "Timer 2 Overflow" },
            { 6 , 1, "Timer 3 Overflow" },
            { 7 , 1, "Serial Communication" },
            { 8 , 1, "DMA 0" },
            { 9 , 1, "DMA 1" },
            { 10, 1, "DMA 2" },
            { 11, 1, "DMA 3" },
            { 12, 1, "Keypad" },
            { 13, 1, "Game Pak (ext)" },
        ]
    }, /* R/W  IE        Interrupt Enable Register */
    register! {
        GBA_IF, "IF",
        [
            { 0 , 1, "LCD V-Blank" },
            { 1 , 1, "LCD H-Blank" },
            { 2 , 1, "LCD V-Counter Match" },
            { 3 , 1, "Timer 0 Overflow" },
            { 4 , 1, "Timer 1 Overflow" },
            { 5 , 1, "Timer 2 Overflow" },
            { 6 , 1, "Timer 3 Overflow" },
            { 7 , 1, "Serial Communication" },
            { 8 , 1, "DMA 0" },
            { 9 , 1, "DMA 1" },
            { 10, 1, "DMA 2" },
            { 11, 1, "DMA 3" },
            { 12, 1, "Keypad" },
            { 13, 1, "Game Pak (ext)" },
        ]
    }, /* R/W  IF        Interrupt Request Flags / IRQ Acknowledge */
    register! {
        GBA_WAITCNT, "WAITCNT",
        [
              { 0 , 2,  "SRAM Wait Control (0..3 = 4,3,2,8 cycles)" },
              { 2 , 2,  "Wait State 0 First Access (0..3 = 4,3,2,8 cycles)" },
              { 4 , 1,  "Wait State 0 Second Access (0..1 = 2,1 cycles)" },
              { 5 , 2,  "Wait State 1 First Access (0..3 = 4,3,2,8 cycles)" },
              { 7 , 1,  "Wait State 1 Second Access (0..1 = 4,1 cycles)" },
              { 8 , 2,  "Wait State 2 First Access (0..3 = 4,3,2,8 cycles)" },
              { 10, 1, "Wait State 2 Second Access (0..1 = 8,1 cycles)" },
              { 11, 2, "PHI Terminal Output (0..3 = Disable, 4.19MHz, 8.38MHz, 16.78MHz)" },
              { 14, 1, "Game Pak Prefetch Buffer (0=Disable, 1=Enable)" },
              { 15, 1, "Game Pak Type Flag (0=GBA, 1=CGB) (IN35 signal)" },
        ]
    }, /* R/W  WAITCNT   Game Pak Waitstate Control */
    register! { GBA_IME, "IME", [] }, /* R/W  IME       Interrupt Master Enable Register */
    register! { GBA_POSTFLG, "POSTFLG", [] }, /* R/W  POSTFLG   Undocumented - Post Boot Flag */
    register! { GBA_HALTCNT, "HALTCNT", [] },
    register! {
        GBA_DISPCNT , "DISPCNT ",
        [
            { 0 , 3, "BG Mode (0-5=Video Mode 0-5, 6-7=Prohibited)" },
            { 3 , 1, "Reserved / CGB Mode (0=GBA, 1=CGB)" },
            { 4 , 1, "Display Frame Select (0-1=Frame 0-1)" },
            { 5 , 1, "H-Blank Interval Free (1=Allow access to OAM during H-Blank)" },
            { 6 , 1, "OBJ Character VRAM Mapping (0=2D, 1=1D" },
            { 7 , 1, "Forced Blank (1=Allow FAST VRAM,Palette,OAM)" },
            { 8 , 1, "Screen Display BG0 (0=Off, 1=On)" },
            { 9 , 1, "Screen Display BG1 (0=Off, 1=On)" },
            { 10, 1, "Screen Display BG2 (0=Off, 1=On)" },
            { 11, 1, "Screen Display BG3 (0=Off, 1=On)" },
            { 12, 1, "Screen Display OBJ (0=Off, 1=On)" },
            { 13, 1, "Window 0 Display Flag (0=Off, 1=On)" },
            { 14, 1, "Window 1 Display Flag (0=Off, 1=On)" },
            { 15, 1, "OBJ Window Display Flag (0=Off, 1=On)" },
        ]
    },
    register! {
        GBA_GREENSWP, "GREENSWP",
        [
            {0, 1, "Green Swap  (0=Normal, 1=Swap)" },
        ]
    }, /* R/W Undocumented - Green Swap */
    register! {
        GBA_DISPSTAT, "DISPSTAT",
        [

            { 0, 1, "V-Blank flag (1=VBlank) (set in line 160..226; not 227" },
            { 1, 1, "H-Blank flag (1=HBlank) (toggled in all lines, 0..227" },
            { 2, 1, "V-Counter flag (1=Match) (set in selected line)" },
            { 3, 1, "V-Blank IRQ Enable (1=Enable)" },
            { 4, 1, "H-Blank IRQ Enable (1=Enable)" },
            { 5, 1, "V-Counter IRQ Enable (1=Enable)" },
            { 6, 1, "DSi: LCD Initialization Ready (0=Busy, 1=Ready" },
            { 7, 1, "NDS: MSB of V-Vcount Setting (LYC.Bit8) (0..262" },
            { 8, 8, "V-Count Setting (LYC) (0..227)" },
        ]
    }, /* R/W General LCD Status (STAT,LYC) */
    register! { GBA_VCOUNT, "VCOUNT  ", [] }, /* R   Vertical Counter (LY) */
    register! {
        GBA_BG0CNT, "BG0CNT  ",
        [
            { 0 , 2, "BG Priority (0-3, 0=Highest)" },
            { 2 , 2, "Character Base Block (0-3, in units of 16 KBytes) (=BG Tile Data)" },
            { 4 , 2, "NDS: MSBs of char base" },
            { 6 , 1, "Mosaic (0=Disable, 1=Enable)" },
            { 7 , 1, "Colors/Palettes (0=16/16, 1=256/1)" },
            { 8 , 5, "Screen Base Block (0-31, in units of 2 KBytes) (=BG Map Data)" },
            { 13, 1, "BG0/BG1: (NDS: Ext Palette ) BG2/BG3: Overflow (0=Transp, 1=Wrap)" },
            { 14, 1, "Screen Size (0-3)" },
        ]
    }, /* R/W BG0 Control */
    register! {
        GBA_BG1CNT, "BG1CNT  ",
        [
            { 0,2 , "BG Priority (0-3, 0=Highest)"},
            { 2,2 , "Character Base Block (0-3, in units of 16 KBytes) (=BG Tile Data)"},
            { 4,2 , "NDS: MSBs of char base"},
            { 6,1 , "Mosaic (0=Disable, 1=Enable)"},
            { 7,1 , "Colors/Palettes (0=16/16, 1=256/1)"},
            { 8,5 , "Screen Base Block (0-31, in units of 2 KBytes) (=BG Map Data)"},
            { 13,1, "BG0/BG1: (NDS: Ext Palette ) BG2/BG3: Overflow (0=Transp, 1=Wrap)"},
            { 14,1, "Screen Size (0-3)"},
        ]
    }, /* R/W BG1 Control */
    register! {
        GBA_BG2CNT, "BG2CNT  ",
        [
            { 0,2 , "BG Priority (0-3, 0=Highest)"},
            { 2,2 , "Character Base Block (0-3, in units of 16 KBytes) (=BG Tile Data)"},
            { 4,2 , "NDS: MSBs of char base"},
            { 6,1 , "Mosaic (0=Disable, 1=Enable)"},
            { 7,1 , "Colors/Palettes (0=16/16, 1=256/1)"},
            { 8,5 , "Screen Base Block (0-31, in units of 2 KBytes) (=BG Map Data)"},
            { 13,1, "BG0/BG1: (NDS: Ext Palette ) BG2/BG3: Overflow (0=Transp, 1=Wrap)"},
            { 14,1, "Screen Size (0-3)"},
        ]
    }, /* R/W BG2 Control */
    register! {
        GBA_BG3CNT, "BG3CNT  ",
        [
            { 0,2 , "BG Priority (0-3, 0=Highest)"},
            { 2,2 , "Character Base Block (0-3, in units of 16 KBytes) (=BG Tile Data)"},
            { 4,2 , "NDS: MSBs of char base"},
            { 6,1 , "Mosaic (0=Disable, 1=Enable)"},
            { 7,1 , "Colors/Palettes (0=16/16, 1=256/1)"},
            { 8,5 , "Screen Base Block (0-31, in units of 2 KBytes) (=BG Map Data)"},
            { 13,1, "BG0/BG1: (NDS: Ext Palette ) BG2/BG3: Overflow (0=Transp, 1=Wrap)"},
            { 14,1, "Screen Size (0-3)"},
        ]
    }, /* R/W BG3 Control */
    register! { GBA_BG0HOFS , "BG0HOFS", [] }, /* W   BG0 X-Offset */
    register! { GBA_BG0VOFS , "BG0VOFS", [] }, /* W   BG0 Y-Offset */
    register! { GBA_BG1HOFS , "BG1HOFS", [] }, /* W   BG1 X-Offset */
    register! { GBA_BG1VOFS , "BG1VOFS", [] }, /* W   BG1 Y-Offset */
    register! { GBA_BG2HOFS , "BG2HOFS", [] }, /* W   BG2 X-Offset */
    register! { GBA_BG2VOFS , "BG2VOFS", [] }, /* W   BG2 Y-Offset */
    register! { GBA_BG3HOFS , "BG3HOFS", [] }, /* W   BG3 X-Offset */
    register! { GBA_BG3VOFS , "BG3VOFS", [] }, /* W   BG3 Y-Offset */
    register! { GBA_BG2PA   , "BG2PA", [] },  /* W   BG2 Rotation/Scaling Parameter A (dx) */
    register! { GBA_BG2PB   , "BG2PB", [] },  /* W   BG2 Rotation/Scaling Parameter B (dmx) */
    register! { GBA_BG2PC   , "BG2PC", [] },  /* W   BG2 Rotation/Scaling Parameter C (dy) */
    register! { GBA_BG2PD   , "BG2PD", [] },  /* W   BG2 Rotation/Scaling Parameter D (dmy) */
    register! { GBA_BG2X    , "BG2X", [] },   /* W   BG2 Reference Point X-Coordinate */
    register! { GBA_BG2Y    , "BG2Y", [] },   /* W   BG2 Reference Point Y-Coordinate */
    register! { GBA_BG3PA   , "BG3PA", [] },  /* W   BG3 Rotation/Scaling Parameter A (dx) */
    register! { GBA_BG3PB   , "BG3PB", [] },  /* W   BG3 Rotation/Scaling Parameter B (dmx) */
    register! { GBA_BG3PC   , "BG3PC", [] },  /* W   BG3 Rotation/Scaling Parameter C (dy) */
    register! { GBA_BG3PD   , "BG3PD", [] },  /* W   BG3 Rotation/Scaling Parameter D (dmy) */
    register! { GBA_BG3X    , "BG3X", [] },   /* W   BG3 Reference Point X-Coordinate */
    register! { GBA_BG3Y    , "BG3Y", [] },   /* W   BG3 Reference Point Y-Coordinate */
    register! {
        GBA_WIN0H, "WIN0H",
        [
            { 0, 8, "X2, Rightmost coordinate of window, plus 1 " },
            { 8, 8,  "X1, Leftmost coordinate of window"},
        ]
    }, /* W   Window 0 Horizontal Dimensions */
    register! {
        GBA_WIN1H, "WIN1H",
        [
            { 0, 8, "X2, Rightmost coordinate of window, plus 1 " },
            { 8, 8, "X1, Leftmost coordinate of window"},
        ]
    }, /* W   Window 1 Horizontal Dimensions */
    register! {
        GBA_WIN0V, "WIN0V",
        [
            {0, 8,  "Y2, Bottom-most coordinate of window, plus 1" },
            {8, 8,  "Y1, Top-most coordinate of window" },
        ]
    }, /* W   Window 0 Vertical Dimensions */
    register! {
        GBA_WIN1V, "WIN1V",
        [
            {0, 8,  "Y2, Bottom-most coordinate of window, plus 1" },
            {8, 8,  "Y1, Top-most coordinate of window" },
        ]
    }, /* W   Window 1 Vertical Dimensions */
    register! {
        GBA_WININ, "WININ",
        [
            { 0 , 1,  "Window 0 BG0 Enable Bits (0=No Display, 1=Display)"},
            { 1 , 1,  "Window 0 BG1 Enable Bits (0=No Display, 1=Display)"},
            { 2 , 1,  "Window 0 BG2 Enable Bits (0=No Display, 1=Display)"},
            { 3 , 1,  "Window 0 BG3 Enable Bits (0=No Display, 1=Display)"},
            { 4 , 1,  "Window 0 OBJ Enable Bit (0=No Display, 1=Display)"},
            { 5 , 1,  "Window 0 Color Special Effect (0=Disable, 1=Enable)"},
            { 8 , 1,  "Window 1 BG0 Enable Bits (0=No Display, 1=Display)"},
            { 9 , 1,  "Window 1 BG1 Enable Bits (0=No Display, 1=Display)"},
            { 10, 1,  "Window 1 BG2 Enable Bits (0=No Display, 1=Display)"},
            { 11, 1,  "Window 1 BG3 Enable Bits (0=No Display, 1=Display)"},
            { 12, 1,  "Window 1 OBJ Enable Bit (0=No Display, 1=Display)"},
            { 13, 1,  "Window 1 Color Special Effect (0=Disable, 1=Enable)"},
        ]
    }, /* R/W Inside of Window 0 and 1 */
    register! {
        GBA_WINOUT, "WINOUT",
        [
            { 0 , 1,  "Window 0 BG0 Enable Bits (0=No Display, 1=Display)"},
            { 1 , 1,  "Window 0 BG1 Enable Bits (0=No Display, 1=Display)"},
            { 2 , 1,  "Window 0 BG2 Enable Bits (0=No Display, 1=Display)"},
            { 3 , 1,  "Window 0 BG3 Enable Bits (0=No Display, 1=Display)"},
            { 4 , 1,  "Window 0 OBJ Enable Bit (0=No Display, 1=Display)"},
            { 5 , 1,  "Window 0 Color Special Effect (0=Disable, 1=Enable)"},
            { 8 , 1,  "Window 1 BG0 Enable Bits (0=No Display, 1=Display)"},
            { 9 , 1,  "Window 1 BG1 Enable Bits (0=No Display, 1=Display)"},
            { 10, 1,  "Window 1 BG2 Enable Bits (0=No Display, 1=Display)"},
            { 11, 1,  "Window 1 BG3 Enable Bits (0=No Display, 1=Display)"},
            { 12, 1,  "Window 1 OBJ Enable Bit (0=No Display, 1=Display)"},
            { 13, 1,  "Window 1 Color Special Effect (0=Disable, 1=Enable)"},
        ]
    }, /* R/W Inside of OBJ Window & Outside of Windows */
    register! {
        GBA_MOSAIC, "MOSAIC",
        [
            { 0, 4, "BG Mosaic H-Size (minus 1)" },
            { 4, 4, "BG Mosaic V-Size (minus 1)" },
            { 8, 4, "OBJ Mosaic H-Size (minus 1)" },
            { 12,4, "OBJ Mosaic V-Size (minus 1)" },
        ]
    }, /* W   Mosaic Size */
    register! {
        GBA_BLDCNT  , "BLDCNT",
        [
            { 0 , 1, "BG0 1st Target Pixel (Background 0)" },
            { 1 , 1, "BG1 1st Target Pixel (Background 1)" },
            { 2 , 1, "BG2 1st Target Pixel (Background 2)" },
            { 3 , 1, "BG3 1st Target Pixel (Background 3)" },
            { 4 , 1, "OBJ 1st Target Pixel (Top-most OBJ pixel)" },
            { 5 , 1, "BD  1st Target Pixel (Backdrop)" },
            { 6 , 2, "Color Effect (0: None 1: Alpha 2: Lighten 3: Darken)" },
            { 8 , 1, "BG0 2nd Target Pixel (Background 0)" },
            { 9 , 1, "BG1 2nd Target Pixel (Background 1)" },
            { 10, 1, "BG2 2nd Target Pixel (Background 2)" },
            { 11, 1, "BG3 2nd Target Pixel (Background 3)" },
            { 12, 1, "OBJ 2nd Target Pixel (Top-most OBJ pixel)" },
            { 13, 1, "BD  2nd Target Pixel (Backdrop)" },
        ]
    }, /* R/W Color Special Effects Selection */
    register! {
        GBA_BLDALPHA, "BLDALPHA",
        [
            {0, 4, "EVA Coef. (1st Target) (0..16 = 0/16..16/16, 17..31=16/16)"},
            {8, 4, "EVB Coef. (2nd Target) (0..16 = 0/16..16/16, 17..31=16/16)"},
        ]
    }, /* R/W Alpha Blending Coefficients */
    register! { GBA_BLDY, "BLDY", [] },       /* W   Brightness (Fade-In/Out) Coefficient */
    // Sound Registers
    register! { GBA_SOUND1CNT_L, "SOUND1CNT_L", [
      {0,3, "Number of sweep shift (n=0-7)"},
      {3,1, "Sweep Frequency Direction (0=Increase, 1=Decrease)"},
      {4,3, "Sweep Time; units of 7.8ms (0-7, min=7.8ms, max=54.7ms)"},
    ] }, /* R/W   Channel 1 Sweep register       (NR10) */
    register! { GBA_SOUND1CNT_H, "SOUND1CNT_H", [
    { 0,6, "Sound length; units of (64-n)/256s (0-63)"},
    { 6,2, "Wave Pattern Duty (0-3, see below)"},
    { 8,3, "Envelope Step-Time; units of n/64s (1-7, 0=No Envelope)"},
    { 11,1, "Envelope Direction (0=Decrease, 1=Increase)"},
    { 12,4, "Initial Volume of envelope (1-15, 0=No Sound)"},
    ] }, /* R/W   Channel 1 Duty/Length/Envelope (NR11, NR12) */
    register! { GBA_SOUND1CNT_X, "SOUND1CNT_X", [
    { 0,11, "Frequency; 131072/(2048-n)Hz (0-2047)"},
    { 14,1,  "Length Flag (1=Stop output when length in NR11 expires)"},
    { 15,1,  "Initial (1=Restart Sound)"},
    ] }, /* R/W   Channel 1 Frequency/Control    (NR13, NR14) */
    register! { GBA_SOUND2CNT_L, "SOUND2CNT_L", [
    { 0,6, "Sound length; units of (64-n)/256s (0-63)"},
    { 6,2, "Wave Pattern Duty (0-3, see below)"},
    { 8,3, "Envelope Step-Time; units of n/64s (1-7, 0=No Envelope)"},
    { 11,1, "Envelope Direction (0=Decrease, 1=Increase)"},
    { 12,4, "Initial Volume of envelope (1-15, 0=No Sound)"},
    ] }, /* R/W   Channel 2 Duty/Length/Envelope (NR21, NR22) */
    register! { GBA_SOUND2CNT_H, "SOUND2CNT_H", [
    { 0 ,11, "Frequency; 131072/(2048-n)Hz (0-2047)"},
    { 14,1,  "Length Flag (1=Stop output when length in NR11 expires)"},
    { 15,1,  "Initial (1=Restart Sound)"},
    ] }, /* R/W   Channel 2 Frequency/Control    (NR23, NR24) */
    register! { GBA_SOUND3CNT_L, "SOUND3CNT_L", [
      { 5, 1, "Wave RAM Dimension (0=One bank, 1=Two banks)" },
      { 6, 1, "Wave RAM Bank Number (0-1, see below)" },
      { 7, 1, "Sound Channel 3 Off (0=Stop, 1=Playback)" },
    ] }, /* R/W   Channel 3 Stop/Wave RAM select (NR30) */
    register! { GBA_SOUND3CNT_H, "SOUND3CNT_H", [
      { 0,8, "Sound length; units of (256-n)/256s (0-255)"},
      { 13,2, "Sound Volume (0=Mute/Zero, 1=100%, 2=50%, 3=25%)"},
      { 15,1, "Force Volume (0=Use above, 1=Force 75% regardless of above)"},
    ] }, /* R/W   Channel 3 Length/Volume        (NR31, NR32) */
    register! { GBA_SOUND3CNT_X, "SOUND3CNT_X", [
      { 0,11, "Sample Rate; 2097152/(2048-n) Hz (0-2047)" },
      { 14,1, "Length Flag (1=Stop output when length in NR31 expires)" },
      { 15,1, "Initial (1=Restart Sound)" },
    ] }, /* R/W   Channel 3 Frequency/Control    (NR33, NR34) */
    register! { GBA_SOUND4CNT_L, "SOUND4CNT_L", [
      { 0, 6, "Sound length; units of (64-n)/256s (0-63)" },
      { 8, 3, "Envelope Step-Time; units of n/64s (1-7, 0=No Envelope)" },
      { 11, 1, "Envelope Direction (0=Decrease, 1=Increase)" },
      { 12, 4, "Initial Volume of envelope (1-15, 0=No Sound)" },
    ] }, /* R/W   Channel 4 Length/Envelope      (NR41, NR42) */
    register! { GBA_SOUND4CNT_H, "SOUND4CNT_H", [
      { 0, 1, "Dividing Ratio of Frequencies (r)"},
      { 3, 1, "Counter Step/Width (0=15 bits, 1=7 bits)"},
      { 4, 1, "Shift Clock Frequency (s)"},
      { 14, 1, "Length Flag (1=Stop output when length in NR41 expires)"},
      { 15, 1, "Initial (1=Restart Sound)"},
    ] }, /* R/W   Channel 4 Frequency/Control    (NR43, NR44) */
    register! { GBA_SOUNDCNT_L , "SOUNDCNT_L", [
      { 0,1, "Sound 1 Master Volume RIGHT" },
      { 1,1, "Sound 2 Master Volume RIGHT" },
      { 2,1, "Sound 3 Master Volume RIGHT" },
      { 3,1, "Sound 4 Master Volume RIGHT" },
      { 4,1, "Sound 1 Master Volume LEFT" },
      { 5,1, "Sound 2 Master Volume LEFT" },
      { 6,1, "Sound 3 Master Volume LEFT" },
      { 7,1, "Sound 4 Master Volume LEFT" },
      { 8,1, "Sound 1 Enable RIGHT" },
      { 9,1, "Sound 2 Enable RIGHT" },
      { 10,1, "Sound 3 Enable RIGHT" },
      { 11,1, "Sound 4 Enable RIGHT" },
      { 12,1, "Sound 1 Enable LEFT" },
      { 13,1, "Sound 2 Enable LEFT" },
      { 14,1, "Sound 3 Enable LEFT" },
      { 15,1, "Sound 4 Enable LEFT" },
    ] }, /* R/W   Control Stereo/Volume/Enable   (NR50, NR51) */
    register! { GBA_SOUNDCNT_H , "SOUNDCNT_H", [
      { 0 ,2, "Sound # 1-4 Volume (0=25%, 1=50%, 2=100%, 3=Prohibited)" },
      { 2 ,1, "DMA Sound A Volume (0=50%, 1=100%)" },
      { 3 ,1, "DMA Sound B Volume (0=50%, 1=100%)" },
      { 8 ,1, "DMA Sound A Enable RIGHT (0=Disable, 1=Enable)" },
      { 9 ,1, "DMA Sound A Enable LEFT (0=Disable, 1=Enable)" },
      { 10,1, "DMA Sound A Timer Select (0=Timer 0, 1=Timer 1)" },
      { 11,1, "DMA Sound A Reset FIFO (1=Reset)" },
      { 12,1, "DMA Sound B Enable RIGHT (0=Disable, 1=Enable)" },
      { 13,1, "DMA Sound B Enable LEFT (0=Disable, 1=Enable)" },
      { 14,1, "DMA Sound B Timer Select (0=Timer 0, 1=Timer 1)" },
      { 15,1, "DMA Sound B Reset FIFO (1=Reset)" },
    ] }, /* R/W   Control Mixing/DMA Control */
    register! { GBA_SOUNDCNT_X , "SOUNDCNT_X", [
      {0, 1, "Sound 1 ON flag (Read Only)" },
      {1, 1, "Sound 2 ON flag (Read Only)" },
      {2, 1, "Sound 3 ON flag (Read Only)" },
      {3, 1, "Sound 4 ON flag (Read Only)" },
      {7, 1, "PSG/FIFO Master Enable (0=Disable, 1=Enable) (Read/Write)" },
    ] }, /* R/W   Control Sound on/off           (NR52) */
    register! {
        GBA_SOUNDBIAS  , "SOUNDBIAS",
        [
            { 1,9,"Bias Level (Default=100h, converting signed samples into unsigned)"},
            { 14,2,"Amplitude Resolution/Sampling Cycle (Default=0, see below)"},
        ]
    }, /* BIOS  Sound PWM Control */
    register! { GBA_WAVE_RAM   , "WAVE_RAM", [] }, /* R/W Channel 3 Wave Pattern RAM (2 banks!!) */
    register! { GBA_FIFO_A     , "FIFO_A", [] },   /* W   Channel A FIFO, Data 0-3 */
    register! { GBA_FIFO_B     , "FIFO_B", [] },   /* W   Channel B FIFO, Data 0-3 */
    // DMA Transfer Channels
    register! { GBA_DMA0SAD  , "DMA0SAD", [] }, /* W    DMA 0 Source Address */
    register! { GBA_DMA0DAD  , "DMA0DAD", [] }, /* W    DMA 0 Destination Address */
    register! { GBA_DMA0CNT_L, "DMA0CNT_L", [] }, /* W    DMA 0 Word Count */
    register! {
        GBA_DMA0CNT_H, "DMA0CNT_H",
        [
            { 5,  2,  "Dest Addr Control (0=Incr,1=Decr,2=Fixed,3=Incr/Reload)" },
            { 7,  2,  "Source Adr Control (0=Incr,1=Decr,2=Fixed,3=Prohibited)" },
            { 9,  1,  "DMA Repeat (0=Off, 1=On) (Must be zero if Bit 11 set)" },
            { 10, 1,  "DMA Transfer Type (0=16bit, 1=32bit)" },
            { 12, 2,  "DMA Start Timing (0=Immediately, 1=VBlank, 2=HBlank, 3=Prohibited)" },
            { 14, 1,  "IRQ upon end of Word Count (0=Disable, 1=Enable)" },
            { 15, 1,  "DMA Enable (0=Off, 1=On)" },
        ]
    }, /* R/W  DMA 0 Control */
    register! { GBA_DMA1SAD  , "DMA1SAD", [] }, /* W    DMA 1 Source Address */
    register! { GBA_DMA1DAD  , "DMA1DAD", [] }, /* W    DMA 1 Destination Address */
    register! { GBA_DMA1CNT_L, "DMA1CNT_L", [] }, /* W    DMA 1 Word Count */
    register! {
        GBA_DMA1CNT_H, "DMA1CNT_H",
        [
            { 5,  2,  "Dest Addr Control (0=Incr,1=Decr,2=Fixed,3=Incr/Reload)" },
            { 7,  2,  "Source Adr Control (0=Incr,1=Decr,2=Fixed,3=Prohibited)" },
            { 9,  1,  "DMA Repeat (0=Off, 1=On) (Must be zero if Bit 11 set)" },
            { 10, 1,  "DMA Transfer Type (0=16bit, 1=32bit)" },
            { 12, 2,  "DMA Start Timing (0=Immediately, 1=VBlank, 2=HBlank, 3=Sound)" },
            { 14, 1,  "IRQ upon end of Word Count (0=Disable, 1=Enable)" },
            { 15, 1,  "DMA Enable (0=Off, 1=On)" },
        ]
    }, /* R/W  DMA 1 Control */
    register! { GBA_DMA2SAD  , "DMA2SAD", [] }, /* W    DMA 2 Source Address */
    register! { GBA_DMA2DAD  , "DMA2DAD", [] }, /* W    DMA 2 Destination Address */
    register! { GBA_DMA2CNT_L, "DMA2CNT_L", [] }, /* W    DMA 2 Word Count */
    register! {
        GBA_DMA2CNT_H, "DMA2CNT_H",
        [
            { 5,  2,  "Dest Addr Control (0=Incr,1=Decr,2=Fixed,3=Incr/Reload)" },
            { 7,  2,  "Source Adr Control (0=Incr,1=Decr,2=Fixed,3=Prohibited)" },
            { 9,  1,  "DMA Repeat (0=Off, 1=On) (Must be zero if Bit 11 set)" },
            { 10, 1,  "DMA Transfer Type (0=16bit, 1=32bit)" },
            { 12, 2,  "DMA Start Timing (0=Immediately, 1=VBlank, 2=HBlank, 3=Sound)" },
            { 14, 1,  "IRQ upon end of Word Count (0=Disable, 1=Enable)" },
            { 15, 1,  "DMA Enable (0=Off, 1=On)" },
        ]
    }, /* R/W  DMA 2 Control */
    register! { GBA_DMA3SAD  , "DMA3SAD", [] }, /* W    DMA 3 Source Address */
    register! { GBA_DMA3DAD  , "DMA3DAD", [] }, /* W    DMA 3 Destination Address */
    register! { GBA_DMA3CNT_L, "DMA3CNT_L", [] }, /* W    DMA 3 Word Count */
    register! {
        GBA_DMA3CNT_H, "DMA3CNT_H",
        [
            { 5,  2,  "Dest Addr Control (0=Incr,1=Decr,2=Fixed,3=Incr/Reload)" },
            { 7,  2,  "Source Adr Control (0=Incr,1=Decr,2=Fixed,3=Prohibited)" },
            { 9,  1,  "DMA Repeat (0=Off, 1=On) (Must be zero if Bit 11 set)" },
            { 10, 1,  "DMA Transfer Type (0=16bit, 1=32bit)" },
            { 11, 1,  "Game Pak DRQ (0=Normal, 1=DRQ <from> Game Pak, DMA3)" },
            { 12, 2,  "DMA Start Timing (0=Immediately, 1=VBlank, 2=HBlank, 3=Video Capture)" },
            { 14, 1,  "IRQ upon end of Word Count (0=Disable, 1=Enable)" },
            { 15, 1,  "DMA Enable (0=Off, 1=On)" },
        ]
    }, /* R/W  DMA 3 Control */
    // Timer Registers
    register! { GBA_TM0CNT_L, "TM0CNT_L", [] }, /* R/W   Timer 0 Counter/Reload */
    register! {
        GBA_TM0CNT_H, "TM0CNT_H",
        [
             { 0 ,2, "Prescaler Selection (0=F/1, 1=F/64, 2=F/256, 3=F/1024)" },
             { 2 ,1, "Count-up (0=Normal, 1=Incr. on prev. Timer overflow)" },
             { 6 ,1, "Timer IRQ Enable (0=Disable, 1=IRQ on Timer overflow)" },
             { 7 ,1, "Timer Start/Stop (0=Stop, 1=Operate)" },
        ]
    }, /* R/W   Timer 0 Control */
    register! { GBA_TM1CNT_L, "TM1CNT_L", [] }, /* R/W   Timer 1 Counter/Reload */
    register! {
        GBA_TM1CNT_H, "TM1CNT_H",
        [
            { 0 ,2, "Prescaler Selection (0=F/1, 1=F/64, 2=F/256, 3=F/1024)" },
            { 2 ,1, "Count-up (0=Normal, 1=Incr. on prev. Timer overflow)" },
            { 6 ,1, "Timer IRQ Enable (0=Disable, 1=IRQ on Timer overflow)" },
            { 7 ,1, "Timer Start/Stop (0=Stop, 1=Operate)" },
        ]
    }, /* R/W   Timer 1 Control */
    register! { GBA_TM2CNT_L, "TM2CNT_L", [] }, /* R/W   Timer 2 Counter/Reload */
    register! {
        GBA_TM2CNT_H, "TM2CNT_H",
        [
            { 0 ,2, "Prescaler Selection (0=F/1, 1=F/64, 2=F/256, 3=F/1024)" },
            { 2 ,1, "Count-up (0=Normal, 1=Incr. on prev. Timer overflow)" },
            { 6 ,1, "Timer IRQ Enable (0=Disable, 1=IRQ on Timer overflow)" },
            { 7 ,1, "Timer Start/Stop (0=Stop, 1=Operate)" },
        ]
    }, /* R/W   Timer 2 Control */
    register! { GBA_TM3CNT_L, "TM3CNT_L", [] }, /* R/W   Timer 3 Counter/Reload */
    register! {
        GBA_TM3CNT_H, "TM3CNT_H",
        [
            { 0 ,2, "Prescaler Selection (0=F/1, 1=F/64, 2=F/256, 3=F/1024)" },
            { 2 ,1, "Count-up (0=Normal, 1=Incr. on prev. Timer overflow)" },
            { 6 ,1, "Timer IRQ Enable (0=Disable, 1=IRQ on Timer overflow)" },
            { 7 ,1, "Timer Start/Stop (0=Stop, 1=Operate)" },
        ]
    }, /* R/W   Timer 3 Control */
    // Serial Communication (1)
    register! { GBA_SIODATA32  , "SIODATA32", [] }, /*R/W   SIO Data (Normal-32bit Mode; shared with below) */
    register! { GBA_SIOMULTI0  , "SIOMULTI0", [] }, /*R/W   SIO Data 0 (Parent)    (Multi-Player Mode) */
    register! { GBA_SIOMULTI1  , "SIOMULTI1", [] }, /*R/W   SIO Data 1 (1st Child) (Multi-Player Mode) */
    register! { GBA_SIOMULTI2  , "SIOMULTI2", [] }, /*R/W   SIO Data 2 (2nd Child) (Multi-Player Mode) */
    register! { GBA_SIOMULTI3  , "SIOMULTI3", [] }, /*R/W   SIO Data 3 (3rd Child) (Multi-Player Mode) */
    register! { GBA_SIOCNT     , "SIOCNT", [] },    /*R/W   SIO Control Register */
    register! { GBA_SIOMLT_SEND, "SIOMLT_SEND", [] }, /*R/W   SIO Data (Local of MultiPlayer; shared below) */
    register! { GBA_SIODATA8   , "SIODATA8", [] }, /*R/W   SIO Data (Normal-8bit and UART Mode) */
    // Keypad Input
    register! {
        GBA_KEYINPUT, "GBA_KEYINPUT",
        [
            { 0, 1, "Button A" },
            { 1, 1, "Button B" },
            { 2, 1, "Select" },
            { 3, 1, "Start" },
            { 4, 1, "Right" },
            { 5, 1, "Left" },
            { 6, 1, "Up" },
            { 7, 1, "Down" },
            { 8, 1, "Button R" },
            { 9, 1, "Button L" },
        ]
    }, /* R      Key Status */
    register! {
        GBA_KEYCNT  , "GBA_KEYCNT",
        [
            { 0, 1, "Button A" },
            { 1, 1, "Button B" },
            { 2, 1, "Select" },
            { 3, 1, "Start" },
            { 4, 1, "Right" },
            { 5, 1, "Left" },
            { 6, 1, "Up" },
            { 7, 1, "Down" },
            { 8, 1, "Button R" },
            { 9, 1, "Button L" },
            { 14,1, "Button IRQ Enable (0=Disable, 1=Enable)" },
            { 15,1, "Button IRQ Condition (0=OR, 1=AND)"},
        ]
    }, /* R/W    Key Interrupt Control */
    // Serial Communication (2)
    register! { GBA_RCNT     , "RCNT", [] }, /* R/W  SIO Mode Select/General Purpose Data */
    register! { GBA_IR       , "IR", [] }, /* -    Ancient - Infrared Register (Prototypes only) */
    register! { GBA_JOYCNT   , "JOYCNT", [] }, /* R/W  SIO JOY Bus Control */
    register! { GBA_JOY_RECV , "JOY_RECV", [] }, /* R/W  SIO JOY Bus Receive Data */
    register! { GBA_JOY_TRANS, "JOY_TRANS", [] }, /* R/W  SIO JOY Bus Transmit Data */
    register! { GBA_JOYSTAT  , "JOYSTAT", [] }, /* R/?  SIO JOY Bus Receive Status */
];
