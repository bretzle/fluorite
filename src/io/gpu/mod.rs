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
    bgcnts: [BgCnt; 4],
    hofs: [Ofs; 4],
    vofs: [Ofs; 4],
    _dxs: [RotationScalingParameter; 2],
    _dmxs: [RotationScalingParameter; 2],
    _dys: [RotationScalingParameter; 2],
    _dmys: [RotationScalingParameter; 2],
    bgxs: [ReferencePointCoord; 2],
    bgys: [ReferencePointCoord; 2],
    bgxs_latch: [ReferencePointCoord; 2],
    bgys_latch: [ReferencePointCoord; 2],
    mosaic: Mosaic,

    // Windows
    win_0_cnt: WindowControl,
    win_1_cnt: WindowControl,
    win_out_cnt: WindowControl,
    win_obj_cnt: WindowControl,

    // Color Special Effects
    bldcnt: BldCnt,
    bldalpha: BldAlpha,
    bldy: Bldy,

    // Palettes
    bg_palettes: [u16; 0x100],
    obj_palettes: [u16; 0x100],

    // Ram
    pub vram: Box<[u8]>,
    pub oam: Box<[u8]>,

    // DMA
    hblank_called: bool,
    vblank_called: bool,

    // Rendering
    rendered_frame: bool,
    dot: u16,
    bg_lines: [[u16; gba::WIDTH]; 4],
    objs_line: [OBJPixel; gba::WIDTH],
    windows_lines: [[bool; gba::WIDTH]; 3],

    pub pixels: Pixels,
    _debug_spec: DebugSpec,
}

impl Gpu {
    const TRANSPARENT_COLOR: u16 = 0x8000;

    pub fn new() -> (Self, DebugSpec) {
        let pixels = vec![0; WIDTH * HEIGHT];
        let debug = Arc::new(Mutex::new(DebugSpecification::new()));

        let gpu = Self {
            dispcnt: Dispcnt::new(),
            green_swap: false,
            dispstat: Dispstat::new(),
            vcount: 0,

            bgcnts: [BgCnt::new(); 4],
            hofs: [Ofs::new(); 4],
            vofs: [Ofs::new(); 4],
            _dxs: [RotationScalingParameter::new(); 2],
            _dmxs: [RotationScalingParameter::new(); 2],
            _dys: [RotationScalingParameter::new(); 2],
            _dmys: [RotationScalingParameter::new(); 2],
            bgxs: [ReferencePointCoord::new(); 2],
            bgys: [ReferencePointCoord::new(); 2],
            bgxs_latch: [ReferencePointCoord::new(); 2],
            bgys_latch: [ReferencePointCoord::new(); 2],
            mosaic: Mosaic::new(),

            bldcnt: BldCnt::new(),
            bldalpha: BldAlpha::new(),
            bldy: Bldy::new(),

            win_0_cnt: WindowControl::new(),
            win_1_cnt: WindowControl::new(),
            win_out_cnt: WindowControl::new(),
            win_obj_cnt: WindowControl::new(),

            bg_palettes: [0; 0x100],
            obj_palettes: [0; 0x100],

            vram: vec![0; 0x18000].into_boxed_slice(),
            oam: vec![0; 0x400].into_boxed_slice(),

            hblank_called: false,
            vblank_called: false,

            rendered_frame: false,
            dot: 0,
            bg_lines: [[0; gba::WIDTH]; 4],
            objs_line: [OBJPixel::none(); gba::WIDTH],
            windows_lines: [[false; gba::WIDTH]; 3],

            pixels,
            _debug_spec: debug.clone(),
        };

        (gpu, debug)
    }

    pub fn read_register(&self, addr: u32) -> u8 {
        assert_eq!(addr >> 12, 0x04000);

        match addr & 0xFFF {
            0x000 => self.dispcnt.read(0),
            0x001 => self.dispcnt.read(1),
            0x002 => self.green_swap as u8,
            0x003 => 0, // Unused area of Green Swap
            0x004 => self.dispstat.read(0),
            0x005 => self.dispstat.read(1),
            0x006 => self.vcount as u8,
            0x007 => 0, // Unused area of VCOUNT
            0x008 => self.bgcnts[0].read(0),
            0x009 => self.bgcnts[0].read(1),
            0x00A => self.bgcnts[1].read(0),
            0x00B => self.bgcnts[1].read(1),
            0x00C => self.bgcnts[2].read(0),
            0x00D => self.bgcnts[2].read(1),
            0x00E => self.bgcnts[3].read(0),
            0x00F => self.bgcnts[3].read(1),
            _ => panic!("Ignoring GPU Read at 0x{:08X}", addr),
        }
    }

    pub fn write_register(&mut self, scheduler: &mut Scheduler, addr: u32, val: u8) {
        assert_eq!(addr >> 12, 0x04000);

        match addr & 0xFFF {
            0x000 => self.dispcnt.write(0, val),
            0x001 => self.dispcnt.write(1, val),
            0x002 => self.green_swap = val & 0x1 != 0,
            0x003 => (),
            0x004 => self.dispstat.write(scheduler, 0, val),
            0x005 => self.dispstat.write(scheduler, 1, val),
            0x006 => (),
            0x007 => (),
            0x008 => self.bgcnts[0].write(0, val),
            0x009 => self.bgcnts[0].write(1, val),
            0x00A => self.bgcnts[1].write(0, val),
            0x00B => self.bgcnts[1].write(1, val),
            0x00C => self.bgcnts[2].write(0, val),
            0x00D => self.bgcnts[2].write(1, val),
            0x00E => self.bgcnts[3].write(0, val),
            0x00F => self.bgcnts[3].write(1, val),
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

    #[inline]
    pub fn parse_oam_addr(addr: u32) -> u32 {
        addr & 0x3FF
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

        // TODO: feature(exclusive_range_pattern)
        match self.dot {
            0..=239 => self.dispstat.remove(DISPSTATFlags::HBLANK), // Visible
            240 => {
                // HBlank
                if self.dispstat.contains(DISPSTATFlags::HBLANK_IRQ_ENABLE) {
                    interrupts.insert(InterruptRequest::HBLANK);
                }
            }
            250 => {
                // TODO: Take into account half
                self.dispstat.insert(DISPSTATFlags::HBLANK);
                if self.vcount < 160 {
                    self.hblank_called = true;
                } // HDMA only occurs on visible scanlines
            }
            _ => {}
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
                self.bgxs_latch = self.bgxs;
                self.bgys_latch = self.bgys;
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
            BGMode::Mode0 => {
                let mut bgs = vec![];
                if self.dispcnt.contains(DISPCNTFlags::DISPLAY_BG0) {
                    bgs.push(0)
                }
                if self.dispcnt.contains(DISPCNTFlags::DISPLAY_BG1) {
                    bgs.push(1)
                }
                if self.dispcnt.contains(DISPCNTFlags::DISPLAY_BG2) {
                    bgs.push(2)
                }
                if self.dispcnt.contains(DISPCNTFlags::DISPLAY_BG3) {
                    bgs.push(3)
                }

                bgs.into_iter().for_each(|bg_i| self.render_text_line(bg_i));
                self.process_lines(0, 3);
            }
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

            BGMode::Mode4 => {
                let (mosaic_x, mosaic_y) = if self.bgcnts[2].mosaic {
                    (
                        self.mosaic.bg_size.h_size as usize,
                        self.mosaic.bg_size.v_size,
                    )
                } else {
                    (1, 1)
                };
                let y = self.vcount / mosaic_y * mosaic_y;
                let start_addr = if self.dispcnt.contains(DISPCNTFlags::DISPLAY_FRAME_SELECT) {
                    0xA000
                } else {
                    0
                } + y as usize * gba::WIDTH;
                for dot_x in 0..gba::WIDTH {
                    let x = dot_x / mosaic_x * mosaic_x;
                    self.bg_lines[2][dot_x] = self.bg_palettes[self.vram[start_addr + x] as usize];
                }
                self.process_lines(2, 2);
            }
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
        let mut pixels = &mut self.pixels;
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
            let mut layers = [Layer::Bd, Layer::Bd];
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
                    layers[0] = Layer::Obj;
                    // Priority is irrelevant so no need to change it
                } else if self.objs_line[dot_x].priority <= priorities[1] {
                    colors[1] = obj_color;
                    layers[1] = Layer::Obj;
                }
            }

            let trans_obj = layers[0] == Layer::Obj && self.objs_line[dot_x].semitransparent;
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
                    ColorSFX::_BrightnessInc => {
                        let mut new_color = 0;
                        for i in (0..3).rev() {
                            let val = colors[0] >> (5 * i) & 0x1F;
                            let new_val = val + (((0x1F - val) * self.bldy.evy as u16) >> 4);
                            new_color = new_color << 5 | new_val & 0x1F;
                        }
                        new_color
                    }
                    ColorSFX::_BrightnessDec => {
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

    fn render_window(&mut self, _window_i: usize) {
        todo!()
    }

    fn render_objs_line(&mut self) {
        todo!()
    }

    fn render_text_line(&mut self, bg_i: usize) {
        let x_offset = self.hofs[bg_i].0 as usize;
        let y_offset = self.vofs[bg_i].0 as usize;
        let bgcnt = self.bgcnts[bg_i];
        let tile_start_addr = bgcnt.tile_block as usize * 0x4000;
        let map_start_addr = bgcnt.map_block as usize * 0x800;
        let bit_depth = if bgcnt.bpp8 { 8 } else { 4 }; // Also bytes per row of tile
        let (mosaic_x, mosaic_y) = if bgcnt.mosaic {
            (
                self.mosaic.bg_size.h_size as usize,
                self.mosaic.bg_size.v_size as usize,
            )
        } else {
            (1, 1)
        };

        let dot_y = self.vcount as usize;
        for dot_x in 0..gba::WIDTH {
            let x = (dot_x + x_offset) / mosaic_x * mosaic_x;
            let y = (dot_y + y_offset) / mosaic_y * mosaic_y;
            // Get Screen Entry
            let mut map_x = x / 8;
            let mut map_y = y / 8;
            let map_start_addr = map_start_addr
                + match bgcnt.screen_size {
                    0 => 0,
                    1 => {
                        if (map_x / 32) % 2 == 1 {
                            0x800
                        } else {
                            0
                        }
                    }
                    2 => {
                        if (map_y / 32) % 2 == 1 {
                            0x800
                        } else {
                            0
                        }
                    }
                    3 => {
                        let x_overflowed = (map_x / 32) % 2 == 1;
                        let y_overflowed = (map_y / 32) % 2 == 1;
                        if x_overflowed && y_overflowed {
                            0x800 * 3
                        } else if y_overflowed {
                            0x800 * 2
                        } else if x_overflowed {
                            0x800
                        } else {
                            0
                        }
                    }
                    _ => unreachable!(),
                };
            map_x %= 32;
            map_y %= 32;
            let addr = map_start_addr + map_y * 32 * 2 + map_x * 2;
            let screen_entry = u16::from_le_bytes([self.vram[addr], self.vram[addr + 1]]) as usize;
            let tile_num = screen_entry & 0x3FF;
            let flip_x = (screen_entry >> 10) & 0x1 != 0;
            let flip_y = (screen_entry >> 11) & 0x1 != 0;
            let palette_num = (screen_entry >> 12) & 0xF;

            // Convert from tile to pixels
            let (palette_num, color_num) = self.get_color_from_tile(
                tile_start_addr,
                tile_num,
                flip_x,
                flip_y,
                bit_depth,
                x % 8,
                y % 8,
                palette_num,
            );
            self.bg_lines[bg_i][dot_x] = if color_num == 0 {
                Self::TRANSPARENT_COLOR
            } else {
                self.bg_palettes[palette_num * 16 + color_num]
            };
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn get_color_from_tile(
        &self,
        tile_start_addr: usize,
        tile_num: usize,
        flip_x: bool,
        flip_y: bool,
        bit_depth: usize,
        tile_x: usize,
        tile_y: usize,
        palette_num: usize,
    ) -> (usize, usize) {
        let addr = tile_start_addr + 8 * bit_depth * tile_num;
        if tile_start_addr < 0x10000 && addr >= 0x10000 {
            return (0, 0);
        } // BG maps can't use OBJ tiles
        let tile_x = if flip_x { 7 - tile_x } else { tile_x };
        let tile_y = if flip_y { 7 - tile_y } else { tile_y };
        let tile = self.vram[addr + tile_y * bit_depth + tile_x / (8 / bit_depth)] as usize;
        if bit_depth == 8 {
            (0, tile)
        } else {
            (palette_num, ((tile >> (4 * (tile_x % 2))) & 0xF))
        }
    }

    pub fn write_palette_ram(&mut self, addr: u32, value: u8) {
        let addr = (addr & 0x3FF) as usize;
        let palettes = if addr < 0x200 {
            &mut self.bg_palettes
        } else {
            &mut self.obj_palettes
        };
        let index = (addr & 0x1FF) / 2;
        if addr % 2 == 0 {
            palettes[index] = palettes[index] & !0x00FF | (value as u16);
        } else {
            palettes[index] = palettes[index] & !0xFF00 | (value as u16) << 8 & !0x8000;
            // Clear high bit
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Layer {
    Bg0 = 0,
    Bg1 = 1,
    Bg2 = 2,
    Bg3 = 3,
    Obj = 4,
    Bd = 5,
}

impl Layer {
    pub fn from(value: usize) -> Layer {
        use Layer::*;
        match value {
            0 => Bg0,
            1 => Bg1,
            2 => Bg2,
            3 => Bg3,
            4 => Obj,
            5 => Bd,
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
