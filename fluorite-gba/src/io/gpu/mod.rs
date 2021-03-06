use self::registers::*;
use super::interrupt_controller::InterruptRequest;
use crate::{
    consts::{HEIGHT, WIDTH},
    gba::Pixels,
};

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
    dxs: [RotationScalingParameter; 2],
    dmxs: [RotationScalingParameter; 2],
    dys: [RotationScalingParameter; 2],
    dmys: [RotationScalingParameter; 2],
    bgxs: [ReferencePointCoord; 2],
    bgys: [ReferencePointCoord; 2],
    bgxs_latch: [ReferencePointCoord; 2],
    bgys_latch: [ReferencePointCoord; 2],
    mosaic: Mosaic,

    // Windows
    winhs: [WindowDimensions; 2],
    winvs: [WindowDimensions; 2],
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
    bg_lines: [[u16; WIDTH]; 4],
    objs_line: [OBJPixel; WIDTH],
    windows_lines: [[bool; WIDTH]; 3],

    pub pixels: Pixels,
}

impl Gpu {
    const TRANSPARENT_COLOR: u16 = 0x8000;
    const OBJ_SIZES: [[(i16, u16); 3]; 4] = [
        [(8, 8), (16, 8), (8, 16)],
        [(16, 16), (32, 8), (8, 32)],
        [(32, 32), (32, 16), (16, 32)],
        [(64, 64), (64, 32), (32, 64)],
    ];

    pub fn new() -> Self {
        Self {
            dispcnt: Dispcnt::new(),
            green_swap: false,
            dispstat: Dispstat::new(),
            vcount: 0,

            bgcnts: [BgCnt::new(); 4],
            hofs: [Ofs::new(); 4],
            vofs: [Ofs::new(); 4],
            dxs: [RotationScalingParameter::new(); 2],
            dmxs: [RotationScalingParameter::new(); 2],
            dys: [RotationScalingParameter::new(); 2],
            dmys: [RotationScalingParameter::new(); 2],
            bgxs: [ReferencePointCoord::new(); 2],
            bgys: [ReferencePointCoord::new(); 2],
            bgxs_latch: [ReferencePointCoord::new(); 2],
            bgys_latch: [ReferencePointCoord::new(); 2],
            mosaic: Mosaic::new(),

            bldcnt: BldCnt::new(),
            bldalpha: BldAlpha::new(),
            bldy: Bldy::new(),

            winhs: [WindowDimensions::new(); 2],
            winvs: [WindowDimensions::new(); 2],
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
            bg_lines: [[0; WIDTH]; 4],
            objs_line: [OBJPixel::none(); WIDTH],
            windows_lines: [[false; WIDTH]; 3],

            pixels: vec![0; WIDTH * HEIGHT],
        }
    }

    pub fn read_register(&self, addr: u32) -> u8 {
        debug_assert_eq!(addr >> 12, 0x04000);

        match addr & 0xFFF {
            0x000 => self.dispcnt.read::<0>(),
            0x001 => self.dispcnt.read::<1>(),
            0x002 => self.green_swap as u8,
            0x003 => 0, // Unused area of Green Swap
            0x004 => self.dispstat.read::<0>(),
            0x005 => self.dispstat.read::<1>(),
            0x006 => self.vcount as u8,
            0x007 => 0, // Unused area of VCOUNT
            0x008 => self.bgcnts[0].read::<0>(),
            0x009 => self.bgcnts[0].read::<1>(),
            0x00A => self.bgcnts[1].read::<0>(),
            0x00B => self.bgcnts[1].read::<1>(),
            0x00C => self.bgcnts[2].read::<0>(),
            0x00D => self.bgcnts[2].read::<1>(),
            0x00E => self.bgcnts[3].read::<0>(),
            0x00F => self.bgcnts[3].read::<1>(),
            0x010 => self.hofs[0].read::<0>(),
            0x011 => self.hofs[0].read::<1>(),
            0x012 => self.vofs[0].read::<0>(),
            0x013 => self.vofs[0].read::<1>(),
            0x014 => self.hofs[1].read::<0>(),
            0x015 => self.hofs[1].read::<1>(),
            0x016 => self.vofs[1].read::<0>(),
            0x017 => self.vofs[1].read::<1>(),
            0x018 => self.hofs[2].read::<0>(),
            0x019 => self.hofs[2].read::<1>(),
            0x01A => self.vofs[2].read::<0>(),
            0x01B => self.vofs[2].read::<1>(),
            0x01C => self.hofs[3].read::<0>(),
            0x01D => self.hofs[3].read::<1>(),
            0x01E => self.vofs[3].read::<0>(),
            0x01F => self.vofs[3].read::<1>(),
            0x020 => self.dxs[0].read(0),
            0x021 => self.dxs[0].read(1),
            0x022 => self.dmxs[0].read(0),
            0x023 => self.dmxs[0].read(1),
            0x024 => self.dys[0].read(0),
            0x025 => self.dys[0].read(1),
            0x026 => self.dmys[0].read(0),
            0x027 => self.dmys[0].read(1),
            0x028 => self.bgxs[0].read(0),
            0x029 => self.bgxs[0].read(1),
            0x02A => self.bgxs[0].read(2),
            0x02B => self.bgxs[0].read(3),
            0x02C => self.bgys[0].read(0),
            0x02D => self.bgys[0].read(1),
            0x02E => self.bgys[0].read(2),
            0x02F => self.bgys[0].read(3),
            0x030 => self.dxs[1].read(0),
            0x031 => self.dxs[1].read(1),
            0x032 => self.dmxs[1].read(0),
            0x033 => self.dmxs[1].read(1),
            0x034 => self.dys[1].read(0),
            0x035 => self.dys[1].read(1),
            0x036 => self.dmys[1].read(0),
            0x037 => self.dmys[1].read(1),
            0x038 => self.bgxs[1].read(0),
            0x039 => self.bgxs[1].read(1),
            0x03A => self.bgxs[1].read(2),
            0x03B => self.bgxs[1].read(3),
            0x03C => self.bgys[1].read(0),
            0x03D => self.bgys[1].read(1),
            0x03E => self.bgys[1].read(2),
            0x03F => self.bgys[1].read(3),
            0x040 => self.winhs[0].read::<0>(),
            0x041 => self.winhs[0].read::<1>(),
            0x042 => self.winhs[1].read::<0>(),
            0x043 => self.winhs[1].read::<1>(),
            0x044 => self.winvs[0].read::<0>(),
            0x045 => self.winvs[0].read::<1>(),
            0x046 => self.winvs[1].read::<0>(),
            0x047 => self.winvs[1].read::<1>(),
            0x048 => self.win_0_cnt.read::<0>(),
            0x049 => self.win_1_cnt.read::<0>(),
            0x04A => self.win_out_cnt.read::<0>(),
            0x04B => self.win_obj_cnt.read::<0>(),
            0x04C => self.mosaic.read::<0>(),
            0x04D => self.mosaic.read::<1>(),
            0x04E => 0,
            0x04F => 0,
            0x050 => self.bldcnt.read::<0>(),
            0x051 => self.bldcnt.read::<1>(),
            0x052 => self.bldalpha.read::<0>(),
            0x053 => self.bldalpha.read::<1>(),
            0x054 => self.bldy.read(0),
            0x055 => self.bldy.read(1),
            0x056..=0x05F => 0,
            _ => panic!("Ignoring GPU Read at 0x{:08X}", addr),
        }
    }

    pub fn write_register(&mut self, addr: u32, val: u8) {
        debug_assert_eq!(addr >> 12, 0x04000);

        match addr & 0xFFF {
            0x000 => self.dispcnt.write::<0>(val),
            0x001 => self.dispcnt.write::<1>(val),
            0x002 => self.green_swap = val & 0x1 != 0,
            0x003 => (),
            0x004 => self.dispstat.write::<0>(val),
            0x005 => self.dispstat.write::<1>(val),
            0x006 => (),
            0x007 => (),
            0x008 => self.bgcnts[0].write::<0>(val),
            0x009 => self.bgcnts[0].write::<1>(val),
            0x00A => self.bgcnts[1].write::<0>(val),
            0x00B => self.bgcnts[1].write::<1>(val),
            0x00C => self.bgcnts[2].write::<0>(val),
            0x00D => self.bgcnts[2].write::<1>(val),
            0x00E => self.bgcnts[3].write::<0>(val),
            0x00F => self.bgcnts[3].write::<1>(val),
            0x010 => self.hofs[0].write::<0>(val),
            0x011 => self.hofs[0].write::<1>(val),
            0x012 => self.vofs[0].write::<0>(val),
            0x013 => self.vofs[0].write::<1>(val),
            0x014 => self.hofs[1].write::<0>(val),
            0x015 => self.hofs[1].write::<1>(val),
            0x016 => self.vofs[1].write::<0>(val),
            0x017 => self.vofs[1].write::<1>(val),
            0x018 => self.hofs[2].write::<0>(val),
            0x019 => self.hofs[2].write::<1>(val),
            0x01A => self.vofs[2].write::<0>(val),
            0x01B => self.vofs[2].write::<1>(val),
            0x01C => self.hofs[3].write::<0>(val),
            0x01D => self.hofs[3].write::<1>(val),
            0x01E => self.vofs[3].write::<0>(val),
            0x01F => self.vofs[3].write::<1>(val),
            0x020 => self.dxs[0].write::<0>(val),
            0x021 => self.dxs[0].write::<1>(val),
            0x022 => self.dmxs[0].write::<0>(val),
            0x023 => self.dmxs[0].write::<1>(val),
            0x024 => self.dys[0].write::<0>(val),
            0x025 => self.dys[0].write::<1>(val),
            0x026 => self.dmys[0].write::<0>(val),
            0x027 => self.dmys[0].write::<1>(val),
            0x028 => {
                self.bgxs[0].write::<0>(val);
                self.bgxs_latch[0] = self.bgxs[0]
            }
            0x029 => {
                self.bgxs[0].write::<1>(val);
                self.bgxs_latch[0] = self.bgxs[0]
            }
            0x02A => {
                self.bgxs[0].write::<2>(val);
                self.bgxs_latch[0] = self.bgxs[0]
            }
            0x02B => {
                self.bgxs[0].write::<3>(val);
                self.bgxs_latch[0] = self.bgxs[0]
            }
            0x02C => {
                self.bgys[0].write::<0>(val);
                self.bgys_latch[0] = self.bgys[0]
            }
            0x02D => {
                self.bgys[0].write::<1>(val);
                self.bgys_latch[0] = self.bgys[0]
            }
            0x02E => {
                self.bgys[0].write::<2>(val);
                self.bgys_latch[0] = self.bgys[0]
            }
            0x02F => {
                self.bgys[0].write::<3>(val);
                self.bgys_latch[0] = self.bgys[0]
            }
            0x030 => self.dxs[1].write::<0>(val),
            0x031 => self.dxs[1].write::<1>(val),
            0x032 => self.dmxs[1].write::<0>(val),
            0x033 => self.dmxs[1].write::<1>(val),
            0x034 => self.dys[1].write::<0>(val),
            0x035 => self.dys[1].write::<1>(val),
            0x036 => self.dmys[1].write::<0>(val),
            0x037 => self.dmys[1].write::<1>(val),
            0x038 => self.bgxs[1].write::<0>(val),
            0x039 => self.bgxs[1].write::<1>(val),
            0x03A => self.bgxs[1].write::<2>(val),
            0x03B => self.bgxs[1].write::<3>(val),
            0x03C => self.bgys[1].write::<0>(val),
            0x03D => self.bgys[1].write::<1>(val),
            0x03E => self.bgys[1].write::<2>(val),
            0x03F => self.bgys[1].write::<3>(val),
            0x040 => self.winhs[0].write::<0>(val),
            0x041 => self.winhs[0].write::<1>(val),
            0x042 => self.winhs[1].write::<0>(val),
            0x043 => self.winhs[1].write::<1>(val),
            0x044 => self.winvs[0].write::<0>(val),
            0x045 => self.winvs[0].write::<1>(val),
            0x046 => self.winvs[1].write::<0>(val),
            0x047 => self.winvs[1].write::<1>(val),
            0x048 => self.win_0_cnt.write::<0>(val),
            0x049 => self.win_1_cnt.write::<0>(val),
            0x04A => self.win_out_cnt.write::<0>(val),
            0x04B => self.win_obj_cnt.write::<0>(val),
            0x04C => self.mosaic.write::<0>(val),
            0x04D => self.mosaic.write::<1>(val),
            0x04E => (),
            0x04F => (),
            0x050 => self.bldcnt.write::<0>(val),
            0x051 => self.bldcnt.write::<1>(val),
            0x052 => self.bldalpha.write::<0>(val),
            0x053 => self.bldalpha.write::<1>(val),
            0x054 => self.bldy.write::<0>(val),
            0x055 => self.bldy.write::<1>(val),
            0x056..=0x05F => (),
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

    pub fn rendered_frame(&mut self) -> bool {
        let rendered_frame = self.rendered_frame;
        self.rendered_frame = false;
        rendered_frame
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
        let mut interrupts = InterruptRequest::new();

        // TODO: feature(exclusive_range_pattern)
        match self.dot {
            0..=239 => self.dispstat.set_hblank(false), // Visible
            240 => {
                // HBlank
                if self.dispstat.hblank_irq_enable() {
                    interrupts.set_hblank(true);
                }
            }
            250 => {
                // TODO: Take into account half
                self.dispstat.set_hblank(true);
                if self.vcount < 160 {
                    self.hblank_called = true;
                } // HDMA only occurs on visible scanlines
            }
            _ => {}
        }

        if self.vcount < 160 && self.vcount != 227 {
            // Visible
            self.dispstat.set_vblank(false);
            if self.dot == 241 {
                self.render_line()
            }
        } else {
            // VBlank
            if self.vcount == 160 && self.dot == 0 {
                self.vblank_called = true;
                if self.dispstat.vblank_irq_enable() {
                    interrupts.set_vblank(true);
                }
            }
            self.dispstat.set_vblank(true);
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
                self.dispstat.set_vcounter(true);
                if self.dispstat.vcounter_irq_enable() {
                    interrupts.set_vcounter_match(true);
                }
            } else {
                self.dispstat.set_vcounter(false);
            }
        }
        interrupts
    }

    fn render_line(&mut self) {
        if self.dispcnt.display_window0() {
            self.render_window(0)
        }
        if self.dispcnt.display_window1() {
            self.render_window(1)
        }
        if self.dispcnt.display_obj() {
            self.render_objs_line()
        }

        match self.dispcnt.mode {
            BGMode::Mode0 => {
                let mut bgs = vec![];
                if self.dispcnt.display_bg0() {
                    bgs.push(0)
                }
                if self.dispcnt.display_bg1() {
                    bgs.push(1)
                }
                if self.dispcnt.display_bg2() {
                    bgs.push(2)
                }
                if self.dispcnt.display_bg3() {
                    bgs.push(3)
                }

                bgs.into_iter().for_each(|bg_i| self.render_text_line(bg_i));
                self.process_lines(0, 3);
            }
            BGMode::Mode1 => {
                let mut bgs = vec![];
                if self.dispcnt.display_bg0() {
                    bgs.push(0)
                }
                if self.dispcnt.display_bg1() {
                    bgs.push(1)
                }
                if self.dispcnt.display_bg2() {
                    bgs.push(2)
                }

                bgs.iter().for_each(|bg_i| {
                    if *bg_i != 2 {
                        self.render_text_line(*bg_i)
                    } else {
                        self.render_affine_line(*bg_i)
                    }
                });
                self.process_lines(0, 2);
            }
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
                for dot_x in 0..WIDTH {
                    let y = self.vcount / mosaic_y * mosaic_y;
                    let x = dot_x / mosaic_x * mosaic_x;
                    let addr = (y as usize * WIDTH + x) * 2;
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
                let start_addr = if self.dispcnt.display_frame_select() {
                    0xA000
                } else {
                    0
                } + y as usize * WIDTH;
                for dot_x in 0..WIDTH {
                    let x = dot_x / mosaic_x * mosaic_x;
                    self.bg_lines[2][dot_x] = self.bg_palettes[self.vram[start_addr + x] as usize];
                }
                self.process_lines(2, 2);
            }
            BGMode::Mode5 => todo!(),
        }
    }

    fn process_lines(&mut self, start_line: usize, end_line: usize) {
        let start_index = self.vcount as usize * WIDTH;

        let mut bgs: Vec<(usize, u8)> = Vec::new();
        for bg_i in start_line..=end_line {
            if self.dispcnt.raw() & (1 << (8 + bg_i)) != 0 {
                bgs.push((bg_i, self.bgcnts[bg_i].priority))
            }
        }
        bgs.sort_by_key(|a| a.1);
        let master_enabled = [
            self.dispcnt.display_bg0(),
            self.dispcnt.display_bg1(),
            self.dispcnt.display_bg2(),
            self.dispcnt.display_bg3(),
            self.dispcnt.display_obj(),
        ];
        let pixels = &mut self.pixels;
        for dot_x in 0..WIDTH {
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

    fn render_window(&mut self, window_i: usize) {
        let y1 = self.winvs[window_i].coord1;
        let y2 = self.winvs[window_i].coord2;
        let y_in_window = if y1 > y2 {
            self.vcount < y1 && self.vcount >= y2
        } else {
            !(y1..y2).contains(&self.vcount)
        };
        if y_in_window {
            for dot_x in 0..WIDTH as u8 {
                self.windows_lines[window_i][dot_x as usize] = false;
            }
            return;
        }

        let x1 = self.winhs[window_i].coord1;
        let x2 = self.winhs[window_i].coord2;
        if x1 > x2 {
            for dot_x in 0..WIDTH as u8 {
                self.windows_lines[window_i][dot_x as usize] = dot_x >= x1 || dot_x < x2;
            }
        } else {
            for dot_x in 0..WIDTH as u8 {
                self.windows_lines[window_i][dot_x as usize] = (x1..x2).contains(&dot_x);
            }
        }
    }

    fn render_objs_line(&mut self) {
        let mut oam_parsed = [[0u16; 3]; 0x80];
        let mut affine_params = [[0u16; 4]; 0x20];
        self.oam
            .chunks(8)
            .enumerate() // 1 OAM Entry, 1 Affine Parameter
            .for_each(|(i, chunk)| {
                oam_parsed[i][0] = u16::from_le_bytes([chunk[0], chunk[1]]);
                oam_parsed[i][1] = u16::from_le_bytes([chunk[2], chunk[3]]);
                oam_parsed[i][2] = u16::from_le_bytes([chunk[4], chunk[5]]);
                affine_params[i / 4][i % 4] = u16::from_le_bytes([chunk[6], chunk[7]]);
            });
        let mut objs = oam_parsed
            .iter()
            .filter(|obj| {
                let obj_shape = (obj[0] >> 14 & 0x3) as usize;
                let obj_size = (obj[1] >> 14 & 0x3) as usize;
                let (_, obj_height) = Self::OBJ_SIZES[obj_size][obj_shape];
                let affine = obj[0] >> 8 & 0x1 != 0;
                let double_size_or_disable = obj[0] >> 9 & 0x1 != 0;
                if !affine && double_size_or_disable {
                    return false;
                }
                let obj_y_bounds = if double_size_or_disable {
                    obj_height * 2
                } else {
                    obj_height
                };

                let obj_y = (obj[0] as u16) & 0xFF;
                let y_end = obj_y + obj_y_bounds;
                let y = self.vcount as u16 + if y_end > 256 { 256 } else { 0 };
                (obj_y..y_end).contains(&y)
            })
            .collect::<Vec<_>>();
        objs.sort_by_key(|a| (*a)[2] >> 10 & 0x3);
        let obj_window_enabled = self.dispcnt.flags.display_obj_window();

        for dot_x in 0..WIDTH {
            self.objs_line[dot_x] = OBJPixel::none();
            self.windows_lines[2][dot_x] = false;
            let mut set_color = false;
            for obj in objs.iter() {
                let obj_shape = (obj[0] >> 14 & 0x3) as usize;
                let obj_size = (obj[1] >> 14 & 0x3) as usize;
                let affine = obj[0] >> 8 & 0x1 != 0;
                let (obj_width, obj_height) = Self::OBJ_SIZES[obj_size][obj_shape];
                let dot_x_signed = (dot_x as i16) / self.mosaic.obj_size.h_size as i16
                    * self.mosaic.obj_size.h_size as i16;
                let obj_x = (obj[1] & 0x1FF) as u16;
                let obj_x = if obj_x & 0x100 != 0 {
                    0xFE00 | obj_x
                } else {
                    obj_x
                } as i16;
                let obj_y = (obj[0] & 0xFF) as u16;
                let double_size = obj[0] >> 9 & 0x1 != 0;
                let obj_x_bounds = if double_size {
                    obj_width * 2
                } else {
                    obj_width
                };
                if !(obj_x..obj_x + obj_x_bounds).contains(&dot_x_signed) {
                    continue;
                }

                let base_tile_num = (obj[2] & 0x3FF) as usize;
                let x_diff = dot_x_signed - obj_x;
                let y = self.vcount / self.mosaic.obj_size.v_size * self.mosaic.obj_size.v_size;
                let y_diff = (y as u16).wrapping_sub(obj_y) & 0xFF;
                let (x_diff, y_diff) = if affine {
                    let (x_diff, y_diff) = if double_size {
                        (
                            x_diff - obj_width / 2,
                            y_diff as i16 - obj_height as i16 / 2,
                        )
                    } else {
                        (x_diff, y_diff as i16)
                    };
                    let aff_param = obj[1] >> 9 & 0x1F;
                    let params = affine_params[aff_param as usize];
                    let (pa, pb, pc, pd) = (
                        RotationScalingParameter::get_float_from_u16(params[0]),
                        RotationScalingParameter::get_float_from_u16(params[1]),
                        RotationScalingParameter::get_float_from_u16(params[2]),
                        RotationScalingParameter::get_float_from_u16(params[3]),
                    );
                    let (x_offset, y_offset) = (obj_width as f64 / 2.0, obj_height as f64 / 2.0);
                    let (x_raw, y_raw) = (
                        pa * (x_diff as f64 - x_offset)
                            + pb * (y_diff as f64 - y_offset)
                            + x_offset,
                        pc * (x_diff as f64 - x_offset)
                            + pd * (y_diff as f64 - y_offset)
                            + y_offset,
                    );
                    if x_raw < 0.0
                        || y_raw < 0.0
                        || x_raw >= obj_width as f64
                        || y_raw >= obj_height as f64
                    {
                        continue;
                    }
                    (x_raw as u16 as i16, y_raw as u16)
                } else {
                    let flip_x = obj[1] >> 12 & 0x1 != 0;
                    let flip_y = obj[1] >> 13 & 0x1 != 0;
                    (
                        if flip_x {
                            obj_width - 1 - x_diff
                        } else {
                            x_diff
                        },
                        if flip_y {
                            obj_height - 1 - y_diff
                        } else {
                            y_diff
                        },
                    )
                };
                let bit_depth = if obj[0] >> 13 & 0x1 != 0 { 8 } else { 4 };
                let base_tile_num = if bit_depth == 8 {
                    base_tile_num / 2
                } else {
                    base_tile_num
                };
                let tile_num = base_tile_num
                    + if self.dispcnt.obj_tiles1d() {
                        (y_diff as i16 / 8 * obj_width + x_diff) / 8
                    } else {
                        y_diff as i16 / 8 * 0x80 / (bit_depth as i16) + x_diff / 8
                    } as usize;
                let tile_x = x_diff % 8;
                let tile_y = y_diff % 8;
                let palette_num = (obj[2] >> 12 & 0xF) as usize;
                // Flipped at tile level, so no need to flip again
                let (palette_num, color_num) = self.get_color_from_tile(
                    0x10000,
                    tile_num,
                    false,
                    false,
                    bit_depth,
                    tile_x as usize,
                    tile_y as usize,
                    palette_num,
                );
                if color_num == 0 {
                    continue;
                }
                let mode = obj[0] >> 10 & 0x3;
                if mode == 2 {
                    self.windows_lines[2][dot_x] = obj_window_enabled;
                    if set_color {
                        break;
                    } // Continue to look for color pixels
                } else if !set_color {
                    self.objs_line[dot_x] = OBJPixel {
                        color: self.obj_palettes[palette_num * 16 + color_num],
                        priority: (obj[2] >> 10 & 0x3) as u8,
                        semitransparent: mode == 1,
                    };
                    set_color = true;
                    // Continue to look for OBJ window pixels if not yet found and window is enabled
                    if self.windows_lines[2][dot_x] || !obj_window_enabled {
                        break;
                    }
                }
            }
        }
    }

    fn render_affine_line(&mut self, bg_i: usize) {
        let mut base_x = self.bgxs_latch[bg_i - 2];
        let mut base_y = self.bgys_latch[bg_i - 2];
        self.bgxs_latch[bg_i - 2] += self.dmxs[bg_i - 2];
        self.bgys_latch[bg_i - 2] += self.dmys[bg_i - 2];
        let dx = self.dxs[bg_i - 2];
        let dy = self.dys[bg_i - 2];
        let bgcnt = self.bgcnts[bg_i];
        let tile_start_addr = bgcnt.tile_block as usize * 0x4000;
        let map_start_addr = bgcnt.map_block as usize * 0x800;
        let map_size = 128 << bgcnt.screen_size; // In Pixels
        let (mosaic_x, mosaic_y) = if bgcnt.mosaic {
            (
                self.mosaic.bg_size.h_size as usize,
                self.mosaic.bg_size.v_size as usize,
            )
        } else {
            (1, 1)
        };

        for dot_x in 0..WIDTH {
            let (x_raw, y_raw) = (base_x.integer(), base_y.integer());
            base_x += dx;
            base_y += dy;
            let (x, y) =
                if x_raw < 0 || x_raw > map_size as i32 || y_raw < 0 || y_raw > map_size as i32 {
                    if bgcnt.wrap {
                        (
                            (x_raw % map_size as i32) as usize,
                            (y_raw % map_size as i32) as usize,
                        )
                    } else {
                        self.bg_lines[bg_i][dot_x] = Self::TRANSPARENT_COLOR;
                        continue;
                    }
                } else {
                    (x_raw as usize, y_raw as usize)
                };
            // Get Screen Entry
            let map_x = (x / mosaic_x * mosaic_x / 8) % (map_size / 8);
            let map_y = (y / mosaic_y * mosaic_y / 8) % (map_size / 8);
            let addr = map_start_addr + map_y * map_size / 8 + map_x;
            let tile_num = self.vram[addr] as usize;

            // Convert from tile to pixels
            let (_, color_num) = self.get_color_from_tile(
                tile_start_addr,
                tile_num,
                false,
                false,
                8,
                x % 8,
                y % 8,
                0,
            );
            self.bg_lines[bg_i][dot_x] = if color_num == 0 {
                Self::TRANSPARENT_COLOR
            } else {
                self.bg_palettes[color_num]
            };
        }
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
        for dot_x in 0..WIDTH {
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

    pub fn read_palette_ram(&self, addr: u32) -> u8 {
        let addr = (addr & 0x3FF) as usize;
        let palettes = if addr < 0x200 {
            &self.bg_palettes
        } else {
            &self.obj_palettes
        };
        let index = (addr & 0x1FF) / 2;
        if addr % 2 == 0 {
            palettes[index] as u8
        } else {
            (palettes[index] >> 8) as u8
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
