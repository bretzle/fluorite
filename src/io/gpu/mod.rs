use self::{debug::DebugSpecification, registers::*};
use super::{interrupt_controller::InterruptRequest, scheduler::Scheduler};
use crate::gba::{self, DebugSpec, Pixels, HEIGHT, WIDTH};
use std::sync::{Arc, Mutex};

pub mod debug;
mod registers;

pub struct Gpu {
    // Registers
    dispcnt: Dispcnt,
    green_swap: bool,
    dispstat: Dispstat,
    vcount: u8,

    // Backgrounds
    bgcnts: [BGCNT; 4],
    mosaic: MOSAIC,

    // Windows
    win_0_cnt: WindowControl,
    win_1_cnt: WindowControl,
    win_out_cnt: WindowControl,
    win_obj_cnt: WindowControl,

    // Color Special Effects
    bldcnt: BLDCNT,
	bldalpha: BLDALPHA,
	bldy: BLDY,

    // Palettes
    bg_palettes: [u16; 0x100],
    obj_palettes: [u16; 0x100],

    // Ram
    pub vram: Box<[u8]>,

    // DMA
    hblank_called: bool,
    vblank_called: bool,

    // Rendering
    rendered_frame: bool,
    dot: u16,
    bg_lines: [[u16; gba::WIDTH]; 4],
    objs_line: [OBJPixel; gba::WIDTH],
    windows_lines: [[bool; gba::WIDTH]; 3],

    pixels: Pixels,
    debug_spec: DebugSpec,
}

impl Gpu {
	const TRANSPARENT_COLOR: u16 = 0x8000;

    pub fn new() -> (Self, Pixels, DebugSpec) {
        let pixels = Arc::new(Mutex::new(vec![0; WIDTH * HEIGHT]));
        let debug = Arc::new(Mutex::new(DebugSpecification::new()));

        let gpu = Self {
            dispcnt: Dispcnt::new(),
            green_swap: false,
            dispstat: Dispstat::new(),
            vcount: 0,

            bgcnts: [BGCNT::new(); 4],
            mosaic: MOSAIC::new(),

			bldcnt: BLDCNT::new(),
			bldalpha: BLDALPHA::new(),
			bldy: BLDY::new(),

			win_0_cnt: WindowControl::new(),
            win_1_cnt: WindowControl::new(),
            win_out_cnt: WindowControl::new(),
            win_obj_cnt: WindowControl::new(),

			bg_palettes: [0; 0x100],
            obj_palettes: [0; 0x100],

            vram: vec![0; 0x18000].into_boxed_slice(),

            hblank_called: false,
            vblank_called: false,

            rendered_frame: false,
            dot: 0,
            bg_lines: [[0; gba::WIDTH]; 4],
            objs_line: [OBJPixel::none(); gba::WIDTH],
            windows_lines: [[false; gba::WIDTH]; 3],

            pixels: pixels.clone(),
            debug_spec: debug.clone(),
        };

        (gpu, pixels, debug)
    }

    pub fn write_register(&mut self, scheduler: &mut Scheduler, addr: u32, val: u8) {
        assert_eq!(addr >> 12, 0x04000);

        match addr & 0xFFF {
            0x000 => self.dispcnt.write(0, val),
            0x001 => self.dispcnt.write(1, val),
            _ => panic!("Ignoring GPU Write 0x{addr:08X} = 0x{val:02X}"),
        }
    }

    #[inline]
    pub fn parse_vram_addr(addr: u32) -> u32 {
        let addr = addr & 0x1FFFF;
        if addr < 0x10000 {
            addr
        } else {
            addr & 0x17FFF
        }
    }

    pub fn hblank_called(&mut self) -> bool {
        let hblank_called = self.hblank_called;
        self.hblank_called = false;
        hblank_called
    }

    pub fn vblank_called(&mut self) -> bool {
        let vblank_called = self.vblank_called;
        self.vblank_called = false;
        vblank_called
    }

    pub fn emulate_dot(&mut self) -> InterruptRequest {
        let mut interrupts = InterruptRequest::empty();
        if self.dot < 240 {
            // Visible
            self.dispstat.remove(DISPSTATFlags::HBLANK);
        } else {
            // HBlank
            if self.dot == 240 {
                if self.dispstat.contains(DISPSTATFlags::HBLANK_IRQ_ENABLE) {
                    interrupts.insert(InterruptRequest::HBLANK);
                }
            }
            if self.dot == 250 {
                // TODO: Take into account half
                self.dispstat.insert(DISPSTATFlags::HBLANK);
                if self.vcount < 160 {
                    self.hblank_called = true
                } // HDMA only occurs on visible scanlines
            }
        }
        if self.vcount < 160 && self.vcount != 227 {
            // Visible
            self.dispstat.remove(DISPSTATFlags::VBLANK);
            if self.dot == 241 {
                self.render_line()
            }
        } else {
            // VBlank
            if self.vcount == 160 && self.dot == 0 {
                self.vblank_called = true;
                if self.dispstat.contains(DISPSTATFlags::VBLANK_IRQ_ENABLE) {
                    interrupts.insert(InterruptRequest::VBLANK)
                }
            }
            self.dispstat.insert(DISPSTATFlags::VBLANK);
        }

        if self.vcount == 160 && self.dot == 0 {
            // TODO: self.tx.send(self.create_debug_windows()).unwrap();
            self.rendered_frame = true;
        }

        self.dot += 1;
        if self.dot == 308 {
            self.dot = 0;
            if self.vcount == 227 {
                // TODO
                // self.bgxs_latch = self.bgxs.clone();
                // self.bgys_latch = self.bgys.clone();
            }
            self.vcount = (self.vcount + 1) % 228;
            if self.vcount == self.dispstat.vcount_setting {
                self.dispstat.insert(DISPSTATFlags::VCOUNTER);
                if self.dispstat.contains(DISPSTATFlags::VCOUNTER_IRQ_ENALBE) {
                    interrupts.insert(InterruptRequest::VCOUNTER_MATCH);
                }
            } else {
                self.dispstat.remove(DISPSTATFlags::VCOUNTER);
            }
        }
        interrupts
    }

    fn render_line(&mut self) {
        if self.dispcnt.contains(DISPCNTFlags::DISPLAY_WINDOW0) {
            self.render_window(0)
        }
        if self.dispcnt.contains(DISPCNTFlags::DISPLAY_WINDOW1) {
            self.render_window(1)
        }
        if self.dispcnt.contains(DISPCNTFlags::DISPLAY_OBJ) {
            self.render_objs_line()
        }

        match self.dispcnt.mode {
            BGMode::Mode0 => todo!(),
            BGMode::Mode1 => todo!(),
            BGMode::Mode2 => todo!(),
            BGMode::Mode3 => {
                let (mosaic_x, mosaic_y) = if self.bgcnts[2].mosaic {
                    (
                        self.mosaic.bg_size.h_size as usize,
                        self.mosaic.bg_size.v_size,
                    )
                } else {
                    (1, 1)
                };
                for dot_x in 0..gba::WIDTH {
                    let y = self.vcount / mosaic_y * mosaic_y;
                    let x = dot_x / mosaic_x * mosaic_x;
                    let addr = (y as usize * gba::WIDTH + x) * 2;
                    self.bg_lines[2][dot_x] =
                        u16::from_le_bytes([self.vram[addr], self.vram[addr + 1]]);
                }
                self.process_lines(2, 2);
            }

            BGMode::Mode4 => todo!(),
            BGMode::Mode5 => todo!(),
        }
    }

    fn process_lines(&mut self, start_line: usize, end_line: usize) {
        let start_index = self.vcount as usize * gba::WIDTH;

        let mut bgs: Vec<(usize, u8)> = Vec::new();
        for bg_i in start_line..=end_line {
            if self.dispcnt.bits() & (1 << (8 + bg_i)) != 0 {
                bgs.push((bg_i, self.bgcnts[bg_i].priority))
            }
        }
        bgs.sort_by_key(|a| a.1);
        let master_enabled = [
            self.dispcnt.contains(DISPCNTFlags::DISPLAY_BG0),
            self.dispcnt.contains(DISPCNTFlags::DISPLAY_BG1),
            self.dispcnt.contains(DISPCNTFlags::DISPLAY_BG2),
            self.dispcnt.contains(DISPCNTFlags::DISPLAY_BG3),
            self.dispcnt.contains(DISPCNTFlags::DISPLAY_OBJ),
        ];
        let mut pixels = self.pixels.lock().unwrap();
        for dot_x in 0..gba::WIDTH {
            let window_control = if self.windows_lines[0][dot_x] {
                self.win_0_cnt
            } else if self.windows_lines[1][dot_x] {
                self.win_1_cnt
            } else if self.windows_lines[2][dot_x] {
                self.win_obj_cnt
            } else if self.dispcnt.windows_enabled() {
                self.win_out_cnt
            } else {
                WindowControl::all()
            };
            self.windows_lines[0][dot_x] = false;
            self.windows_lines[1][dot_x] = false;
            self.windows_lines[2][dot_x] = false;
            let enabled = [
                master_enabled[0] && window_control.bg0_enable,
                master_enabled[1] && window_control.bg1_enable,
                master_enabled[2] && window_control.bg2_enable,
                master_enabled[3] && window_control.bg3_enable,
                master_enabled[4] && window_control.obj_enable,
            ];

            // Store top 2 layers
            let mut colors = [self.bg_palettes[0], self.bg_palettes[0]]; // Default is backdrop color
            let mut layers = [Layer::BD, Layer::BD];
            let mut priorities = [4, 4];
            let mut i = 0;
            for (bg_i, priority) in bgs.iter() {
                let color = self.bg_lines[*bg_i][dot_x];
                if color != Gpu::TRANSPARENT_COLOR && enabled[*bg_i] {
                    colors[i] = color;
                    layers[i] = Layer::from(*bg_i);
                    priorities[i] = *priority;
                    if i == 0 {
                        i += 1
                    } else {
                        break;
                    }
                }
            }
            let obj_color = self.objs_line[dot_x].color;
            if enabled[4] && obj_color != Gpu::TRANSPARENT_COLOR {
                if self.objs_line[dot_x].priority <= priorities[0] {
                    colors[1] = colors[0];
                    layers[1] = layers[0];
                    colors[0] = obj_color;
                    layers[0] = Layer::OBJ;
                    // Priority is irrelevant so no need to change it
                } else if self.objs_line[dot_x].priority <= priorities[1] {
                    colors[1] = obj_color;
                    layers[1] = Layer::OBJ;
                }
            }

            let trans_obj = layers[0] == Layer::OBJ && self.objs_line[dot_x].semitransparent;
            let target1_enabled =
                self.bldcnt.target_pixel1.enabled[layers[0] as usize] || trans_obj;
            let target2_enabled = self.bldcnt.target_pixel2.enabled[layers[1] as usize];
            let final_color = if window_control.color_special_enable && target1_enabled {
                let effect = if trans_obj && target2_enabled {
                    ColorSFX::AlphaBlend
                } else {
                    self.bldcnt.effect
                };
                match effect {
                    ColorSFX::None => colors[0],
                    ColorSFX::AlphaBlend => {
                        if target2_enabled {
                            let mut new_color = 0;
                            for i in (0..3).rev() {
                                let val1 = colors[0] >> (5 * i) & 0x1F;
                                let val2 = colors[1] >> (5 * i) & 0x1F;
                                let new_val = std::cmp::min(
                                    0x1F,
                                    (val1 * self.bldalpha.eva + val2 * self.bldalpha.evb) >> 4,
                                );
                                new_color = new_color << 5 | new_val;
                            }
                            new_color
                        } else {
                            colors[0]
                        }
                    }
                    ColorSFX::BrightnessInc => {
                        let mut new_color = 0;
                        for i in (0..3).rev() {
                            let val = colors[0] >> (5 * i) & 0x1F;
                            let new_val = val + (((0x1F - val) * self.bldy.evy as u16) >> 4);
                            new_color = new_color << 5 | new_val & 0x1F;
                        }
                        new_color
                    }
                    ColorSFX::BrightnessDec => {
                        let mut new_color = 0;
                        for i in (0..3).rev() {
                            let val = colors[0] >> (5 * i) & 0x1F;
                            let new_val = val - ((val * self.bldy.evy as u16) >> 4);
                            new_color = new_color << 5 | new_val & 0x1F;
                        }
                        new_color
                    }
                }
            } else {
                colors[0]
            };
            pixels[start_index + dot_x] = final_color;
        }
    }

    fn render_window(&mut self, window_i: usize) {
        todo!()
    }

    fn render_objs_line(&mut self) {
        todo!()
    }
}


#[derive(Clone, Copy, PartialEq)]
enum Layer {
    BG0 = 0,
    BG1 = 1,
    BG2 = 2,
    BG3 = 3,
    OBJ = 4,
    BD = 5,
}

impl Layer {
    pub fn from(value: usize) -> Layer {
        use Layer::*;
        match value {
            0 => BG0,
            1 => BG1,
            2 => BG2,
            3 => BG3,
            4 => OBJ,
            5 => BD,
            _ => unreachable!(),
        }
    }
}

#[derive(Clone, Copy)]
struct OBJPixel {
    color: u16,
    priority: u8,
    semitransparent: bool,
}

impl OBJPixel {
    pub fn none() -> Self {
        Self {
            color: Gpu::TRANSPARENT_COLOR,
            priority: 4,
            semitransparent: false,
        }
    }
}