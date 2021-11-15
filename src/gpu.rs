use self::render::Point;
use self::window::{Window, WindowInfo, WindowType};
use crate::dma::{DmaNotifier, TIMING_HBLANK, TIMING_VBLANK};
use crate::gba::NUM_RENDER_TIMES;
use crate::gpu::render::{utils, SCREEN_VIEWPORT};
use crate::interrupt::{Interrupt, SharedInterruptFlags};
use crate::sched::{GpuEvent, Scheduler};
use crate::sysbus::Bus;
use crate::{consts::*, index2d, interrupt, GpuMemoryMappedIO, VideoInterface};
use arrayvec::ArrayVec;
use fluorite_arm::Addr;
use fluorite_common::{CircularBuffer, Shared};
use modular_bitfield::prelude::*;
use static_assertions::assert_eq_size;
use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;
use std::time::{Duration, Instant};

mod render;
mod window;

pub use window::WindowFlags;

pub struct Gpu {
    interrupt_flags: SharedInterruptFlags,
    scheduler: Shared<Scheduler>,

    pub vcount: usize,
    pub dispcnt: DisplayControl,
    pub dispstat: DisplayStatus,

    pub bgcnt: [BgControl; 4],
    pub bg_vofs: [u16; 4],
    pub bg_hofs: [u16; 4],
    pub bg_aff: [BgAffine; 2],
    pub win0: Window,
    pub win1: Window,
    pub winout_flags: WindowFlags,
    pub winobj_flags: WindowFlags,
    pub mosaic: RegMosaic,
    pub bldcnt: BlendControl,
    pub bldalpha: BlendAlpha,
    pub bldy: u16,
    pub palette_ram: Box<[u8]>,
    pub vram: Box<[u8]>,
    pub oam: Box<[u8]>,

    pub(crate) vram_obj_tiles_start: u32,
    pub(crate) obj_buffer: Box<[ObjBufferEntry]>,
    pub(crate) frame_buffer: Box<[u32]>,
    pub(crate) bg_line: [Box<[Rgb15]>; 4],

    pub render_times: CircularBuffer<Duration, NUM_RENDER_TIMES>,
    current_frame_time: Duration,
}

impl Gpu {
    pub fn new(mut scheduler: Shared<Scheduler>, interrupt_flags: SharedInterruptFlags) -> Self {
        scheduler.push_gpu_event(GpuEvent::HDraw, CYCLES_HDRAW);

        Self {
            interrupt_flags,
            scheduler,

            vcount: 0,
            dispcnt: DisplayControl::from(0x80),
            dispstat: DisplayStatus::default(),

            bg_aff: [BgAffine::default(); 2],

            bgcnt: Default::default(),
            bldcnt: BlendControl::default(),
            palette_ram: vec![0; 1 * 1024].into_boxed_slice(),
            vram: vec![0; 128 * 1024].into_boxed_slice(),
            oam: vec![0; 1 * 1024].into_boxed_slice(),

            vram_obj_tiles_start: VRAM_OBJ_TILES_START_TEXT,
            obj_buffer: vec![Default::default(); DISPLAY_WIDTH * DISPLAY_HEIGHT].into_boxed_slice(),
            frame_buffer: vec![0; DISPLAY_WIDTH * DISPLAY_HEIGHT].into_boxed_slice(),
            bg_line: [
                vec![Rgb15::TRANSPARENT; DISPLAY_WIDTH].into_boxed_slice(),
                vec![Rgb15::TRANSPARENT; DISPLAY_WIDTH].into_boxed_slice(),
                vec![Rgb15::TRANSPARENT; DISPLAY_WIDTH].into_boxed_slice(),
                vec![Rgb15::TRANSPARENT; DISPLAY_WIDTH].into_boxed_slice(),
            ],

            render_times: CircularBuffer::new([Duration::MAX; NUM_RENDER_TIMES]),
            current_frame_time: Duration::ZERO,
            bg_vofs: [0; 4],
            bg_hofs: [0; 4],
            win0: Window::default(),
            win1: Window::default(),
            winout_flags: WindowFlags::from(0),
            winobj_flags: WindowFlags::from(0),
            mosaic: RegMosaic::default(),
            bldalpha: BlendAlpha::default(),
            bldy: 0,
        }
    }

    pub fn write_dispcnt(&mut self, val: u16) {
        let old = self.dispcnt.mode;
        self.dispcnt.write(val);
        let new = self.dispcnt.mode;

        if old != new {
            println!("[GPU] Display mode changed! {} -> {}", old, new);
            self.vram_obj_tiles_start = if new as u8 >= 3 {
                VRAM_OBJ_TILES_START_BITMAP
            } else {
                VRAM_OBJ_TILES_START_TEXT
            };
        }
    }

    pub fn on_event<T, D>(
        &mut self,
        event: GpuEvent,
        extra_cycles: usize,
        notifier: &mut D,
        device: &Rc<RefCell<T>>,
    ) where
        T: VideoInterface,
        D: DmaNotifier,
    {
        let now = Instant::now();
        let (next_event, cycles) = match event {
            GpuEvent::HDraw => self.handle_hdraw_end(notifier),
            GpuEvent::HBlank => self.handle_hblank_end(notifier, device),
            GpuEvent::VBlankHDraw => self.handle_vblank_hdraw_end(),
            GpuEvent::VBlankHBlank => self.handle_vblank_hblank_end(),
        };
        self.scheduler
            .push_gpu_event(next_event, cycles - extra_cycles);

        if self.vcount != DISPLAY_HEIGHT {
            self.current_frame_time += now.elapsed();
        } else {
            self.render_times.push(self.current_frame_time);
            self.current_frame_time = Duration::ZERO;
        }
    }

    fn handle_hdraw_end<D: DmaNotifier>(&mut self, notifier: &mut D) -> (GpuEvent, usize) {
        self.dispstat.set_hblank_flag(true);
        if self.dispstat.hblank_irq_enable() {
            interrupt::signal_irq(&self.interrupt_flags, Interrupt::LcdHBlank);
        };
        notifier.notify(TIMING_HBLANK);

        // Next event
        (GpuEvent::HBlank, CYCLES_HBLANK)
    }

    fn handle_hblank_end<T, D>(
        &mut self,
        notifier: &mut D,
        device: &RefCell<T>,
    ) -> (GpuEvent, usize)
    where
        T: VideoInterface,
        D: DmaNotifier,
    {
        self.update_vcount(self.vcount + 1);

        if self.vcount < DISPLAY_HEIGHT {
            self.dispstat.set_hblank_flag(false);
            self.render_scanline();
            // update BG2/3 reference points on the end of a scanline
            for i in 0..2 {
                self.bg_aff[i].internal_x += self.bg_aff[i].pb as i16 as i32;
                self.bg_aff[i].internal_y += self.bg_aff[i].pd as i16 as i32;
            }

            (GpuEvent::HDraw, CYCLES_HDRAW)
        } else {
            // latch BG2/3 reference points on vblank
            for i in 0..2 {
                self.bg_aff[i].internal_x = self.bg_aff[i].x;
                self.bg_aff[i].internal_y = self.bg_aff[i].y;
            }

            self.dispstat.set_vblank_flag(true);
            self.dispstat.set_hblank_flag(false);
            if self.dispstat.vblank_irq_enable() {
                interrupt::signal_irq(&self.interrupt_flags, Interrupt::LcdVBlank);
            };

            notifier.notify(TIMING_VBLANK);

            device.borrow_mut().render(&self.frame_buffer);

            self.obj_buffer_reset();

            (GpuEvent::VBlankHDraw, CYCLES_HDRAW)
        }
    }

    fn handle_vblank_hdraw_end(&mut self) -> (GpuEvent, usize) {
        self.dispstat.set_hblank_flag(true);
        if self.dispstat.hblank_irq_enable() {
            interrupt::signal_irq(&self.interrupt_flags, Interrupt::LcdHBlank);
        };
        (GpuEvent::VBlankHBlank, CYCLES_HBLANK)
    }

    fn handle_vblank_hblank_end(&mut self) -> (GpuEvent, usize) {
        if self.vcount < DISPLAY_HEIGHT + VBLANK_LINES - 1 {
            self.update_vcount(self.vcount + 1);
            self.dispstat.set_hblank_flag(false);
            (GpuEvent::VBlankHDraw, CYCLES_HDRAW)
        } else {
            self.update_vcount(0);
            self.dispstat.set_vblank_flag(false);
            self.dispstat.set_hblank_flag(false);
            self.render_scanline();
            (GpuEvent::HDraw, CYCLES_HDRAW)
        }
    }

    fn update_vcount(&mut self, value: usize) {
        self.vcount = value;
        let vcount_setting = self.dispstat.vcount_setting();
        self.dispstat
            .set_vcount_flag(vcount_setting as usize == self.vcount);

        if self.dispstat.vcount_irq_enable() && self.dispstat.vcount_flag() {
            interrupt::signal_irq(&self.interrupt_flags, Interrupt::LcdVCounterMatch);
        }
    }

    pub fn render_scanline(&mut self) {
        if self.dispcnt.force_blank {
            for x in self.frame_buffer[self.vcount * DISPLAY_WIDTH..]
                .iter_mut()
                .take(DISPLAY_WIDTH)
            {
                *x = 0xf8f8f8;
            }
            return;
        }

        if self.dispcnt.enable_obj {
            todo!()
            // self.render_objs();
        }

        match self.dispcnt.mode {
            0 => {
                todo!();
                // if self.dispcnt.enable_bg0() {
                //     self.render_reg_bg(0);
                // }
                // if self.dispcnt.enable_bg1() {
                //     self.render_reg_bg(0);
                // }
                // if self.dispcnt.enable_bg2() {
                //     self.render_reg_bg(0);
                // }
                // if self.dispcnt.enable_bg3() {
                //     self.render_reg_bg(0);
                // }
                // self.finalize_scanline(0, 3);
            }
            1 => {
                todo!();
                // if self.dispcnt.enable_bg2() {
                //     self.render_aff_bg(2);
                // }
                // if self.dispcnt.enable_bg1() {
                //     self.render_reg_bg(1);
                // }
                // if self.dispcnt.enable_bg0() {
                //     self.render_reg_bg(0);
                // }
                // self.finalize_scanline(0, 2);
            }
            2 => {
                todo!();
                // if self.dispcnt.enable_bg3() {
                //     self.render_aff_bg(3);
                // }
                // if self.dispcnt.enable_bg2() {
                //     self.render_aff_bg(2);
                // }
                // self.finalize_scanline(2, 3);
            }
            3 => {
                self.render_mode3(2);
                self.finalize_scanline(2, 2);
            }
            4 => {
                self.render_mode4(2);
                self.finalize_scanline(2, 2);
            }
            5 => {
                todo!();
                // self.render_mode5(2);
                // self.finalize_scanline(2, 2);
            }
            other => panic!("{}", other),
        }
    }

    /// Clears the gpu obj buffer
    pub fn obj_buffer_reset(&mut self) {
        for x in self.obj_buffer.iter_mut() {
            *x = Default::default();
        }
    }

    pub fn finalize_scanline(&mut self, bg_start: usize, bg_end: usize) {
        let backdrop_color = Rgb15(self.palette_ram.read_16(0));

        // filter out disabled backgrounds and sort by priority
        // the backgrounds are sorted once for the entire scanline
        let mut sorted_backgrounds: ArrayVec<[usize; 4]> = (bg_start..=bg_end)
            .filter(|bg| self.dispcnt.enable_bg[*bg])
            .collect();
        sorted_backgrounds.sort_by_key(|bg| (self.bgcnt[*bg].priority, *bg));

        let y = self.vcount;

        if !self.dispcnt.is_using_windows() {
            for x in 0..DISPLAY_WIDTH {
                let win = WindowInfo::new(WindowType::WinNone, WindowFlags::all());
                self.finalize_pixel(x, y, &win, &sorted_backgrounds, backdrop_color);
            }
        } else {
            todo!();
        }
    }

    fn render_mode3(&mut self, bg: usize) {
        let _y = self.vcount;

        let pa = self.bg_aff[bg - 2].pa as i32;
        let pc = self.bg_aff[bg - 2].pc as i32;
        let ref_point = self.get_ref_point(bg);

        let wraparound = self.bgcnt[bg].affine_wraparound;

        for x in 0..DISPLAY_WIDTH {
            let mut t = utils::transform_bg_point(ref_point, x as i32, pa, pc);
            if !SCREEN_VIEWPORT.contains_point(t) {
                if wraparound {
                    t.0 = t.0.rem_euclid(SCREEN_VIEWPORT.w);
                    t.1 = t.1.rem_euclid(SCREEN_VIEWPORT.h);
                } else {
                    self.bg_line[bg][x] = Rgb15::TRANSPARENT;
                    continue;
                }
            }
            let pixel_index = index2d!(u32, t.0, t.1, DISPLAY_WIDTH);
            let pixel_ofs = 2 * pixel_index;
            let color = Rgb15(self.vram.read_16(pixel_ofs));
            self.bg_line[bg][x] = color;
        }
    }

    fn render_mode4(&mut self, bg: usize) {
        let page_ofs = match self.dispcnt.display_frame_select {
            0 => 0x0600_0000 - VRAM_ADDR,
            1 => 0x0600_a000 - VRAM_ADDR,
            _ => unreachable!(),
        };

        let _y = self.vcount;

        let pa = self.bg_aff[bg - 2].pa as i32;
        let pc = self.bg_aff[bg - 2].pc as i32;
        let ref_point = self.get_ref_point(bg);

        let wraparound = self.bgcnt[bg].affine_wraparound;

        for x in 0..DISPLAY_WIDTH {
            let mut t = utils::transform_bg_point(ref_point, x as i32, pa, pc);
            if !SCREEN_VIEWPORT.contains_point(t) {
                if wraparound {
                    t.0 = t.0.rem_euclid(SCREEN_VIEWPORT.w);
                    t.1 = t.1.rem_euclid(SCREEN_VIEWPORT.h);
                } else {
                    self.bg_line[bg][x] = Rgb15::TRANSPARENT;
                    continue;
                }
            }
            let bitmap_index = index2d!(u32, t.0, t.1, DISPLAY_WIDTH);
            let bitmap_ofs = page_ofs + (bitmap_index as u32);
            let index = self.vram.read_8(bitmap_ofs) as u32;
            let color = self.get_palette_color(index, 0, 0);
            self.bg_line[bg][x] = color;
        }
    }

    pub fn get_ref_point(&self, bg: usize) -> Point {
        assert!(bg == 2 || bg == 3);
        (
            self.bg_aff[bg - 2].internal_x,
            self.bg_aff[bg - 2].internal_y,
        )
    }

    pub fn get_palette_color(&mut self, index: u32, palette_bank: u32, offset: u32) -> Rgb15 {
        if index == 0 || (palette_bank != 0 && index % 16 == 0) {
            return Rgb15::TRANSPARENT;
        }
        let value = self
            .palette_ram
            .read_16(offset + 2 * index + 0x20 * palette_bank);

        // top bit is ignored
        Rgb15(value & 0x7FFF)
    }

    fn finalize_pixel(
        &mut self,
        x: usize,
        y: usize,
        win: &WindowInfo,
        backgrounds: &[usize],
        backdrop_color: Rgb15,
    ) {
        let output = unsafe {
            let ptr = self.frame_buffer[y * DISPLAY_WIDTH..].as_mut_ptr();
            std::slice::from_raw_parts_mut(ptr, DISPLAY_WIDTH)
        };

        // The backdrop layer is the default
        let backdrop_layer = RenderLayer::backdrop(backdrop_color);

        // Backgrounds are already sorted
        // lets start by taking the first 2 backgrounds that have an opaque pixel at x
        let mut it = backgrounds
            .iter()
            .filter(|i| !self.bg_line[**i][x].is_transparent())
            .take(2);

        let mut top_layer = it.next().map_or(backdrop_layer, |bg| {
            RenderLayer::background(*bg, self.bg_line[*bg][x], self.bgcnt[*bg].priority)
        });

        let mut bot_layer = it.next().map_or(backdrop_layer, |bg| {
            RenderLayer::background(*bg, self.bg_line[*bg][x], self.bgcnt[*bg].priority)
        });

        drop(it);

        // Now that backgrounds are taken care of, we need to check if there is an object pixel that takes priority of one of the layers
        let obj_entry = self.obj_buffer_get(x, y);
        if win.flags.obj_enabled() && self.dispcnt.enable_obj && !obj_entry.color.is_transparent() {
            let obj_layer = RenderLayer::objects(obj_entry.color, obj_entry.priority);
            if obj_layer.priority <= top_layer.priority {
                bot_layer = top_layer;
                top_layer = obj_layer;
            } else if obj_layer.priority <= bot_layer.priority {
                bot_layer = obj_layer;
            }
        }

        let obj_entry = self.obj_buffer_get(x, y);
        let obj_alpha_blend = top_layer.is_object() && obj_entry.alpha;

        let top_flags = self.bldcnt.target1;
        let bot_flags = self.bldcnt.target2;

        let sfx_enabled = (self.bldcnt.mode != BlendMode::BldNone || obj_alpha_blend)
            && top_flags.contains_render_layer(&top_layer); // sfx must at least have a first target configured

        if win.flags.sfx_enabled() && sfx_enabled {
            todo!()
        } else {
            // no blending, just use the top pixel
            output[x] = top_layer.pixel.to_rgb24();
        }
    }

    fn obj_buffer_get(&self, x: usize, y: usize) -> &ObjBufferEntry {
        &self.obj_buffer[index2d!(x, y, DISPLAY_WIDTH)]
    }

    pub fn skip_bios(&mut self) {
        for i in 0..2 {
            self.bg_aff[i].pa = 0x100;
            self.bg_aff[i].pb = 0;
            self.bg_aff[i].pc = 0;
            self.bg_aff[i].pd = 0x100;
        }
    }
}

impl Bus for Gpu {
    fn read_8(&mut self, _addr: Addr) -> u8 {
        todo!()
    }

    fn write_8(&mut self, _addr: Addr, _val: u8) {
        todo!()
    }

    fn write_16(&mut self, addr: Addr, val: u16) {
        let page = addr as usize >> 24;

        match page {
            PAGE_PALRAM => self.palette_ram.write_16(addr & 0x3FE, val),
            PAGE_VRAM => {
                let mut ofs = addr & ((VIDEO_RAM_SIZE as u32) - 1);
                if ofs > 0x18000 {
                    ofs -= 0x8000;
                }
                self.vram.write_16(ofs, val)
            }
            PAGE_OAM => self.oam.write_16(addr & 0x3FE, val),
            _ => unreachable!("{addr} ({page})"),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct DisplayControl {
    pub mode: u16,
    pub display_frame_select: u16,
    pub hblank_interval_free: bool,
    pub obj_character_vram_mapping: bool,
    pub force_blank: bool,
    pub enable_bg: [bool; 4],
    pub enable_obj: bool,
    pub enable_window0: bool,
    pub enable_window1: bool,
    pub enable_obj_window: bool,
}

impl From<u16> for DisplayControl {
    fn from(value: u16) -> DisplayControl {
        let mut dispcnt = DisplayControl::default();
        dispcnt.write(value);
        dispcnt
    }
}

impl GpuMemoryMappedIO for DisplayControl {
    #[inline]
    fn write(&mut self, value: u16) {
        self.mode = value & 0b111;
        self.display_frame_select = (value >> 4) & 1;
        self.hblank_interval_free = (value >> 5) & 1 != 0;
        self.obj_character_vram_mapping = (value >> 6) & 1 != 0;
        self.force_blank = (value >> 7) & 1 != 0;
        self.enable_bg[0] = (value >> 8) & 1 != 0;
        self.enable_bg[1] = (value >> 9) & 1 != 0;
        self.enable_bg[2] = (value >> 10) & 1 != 0;
        self.enable_bg[3] = (value >> 11) & 1 != 0;
        self.enable_obj = (value >> 12) & 1 != 0;
        self.enable_window0 = (value >> 13) & 1 != 0;
        self.enable_window1 = (value >> 14) & 1 != 0;
        self.enable_obj_window = (value >> 15) & 1 != 0;
    }

    #[inline]
    fn read(&self) -> u16 {
        self.mode
            | self.display_frame_select << 4
            | u16::from(self.hblank_interval_free) << 5
            | u16::from(self.obj_character_vram_mapping) << 6
            | u16::from(self.force_blank) << 7
            | u16::from(self.enable_bg[0]) << 8
            | u16::from(self.enable_bg[1]) << 9
            | u16::from(self.enable_bg[2]) << 10
            | u16::from(self.enable_bg[3]) << 11
            | u16::from(self.enable_obj) << 12
            | u16::from(self.enable_window0) << 13
            | u16::from(self.enable_window1) << 14
            | u16::from(self.enable_obj_window) << 15
    }
}

impl DisplayControl {
    pub fn is_using_windows(&self) -> bool {
        self.enable_window0 || self.enable_window1 || self.enable_obj_window
    }
}

#[derive(BitfieldSpecifier, Copy, Clone, Debug, PartialEq)]
#[bits = 3]
#[repr(u8)]
pub enum LcdMode {
    Mode0 = 0b000,
    Mode1 = 0b001,
    Mode2 = 0b010,
    Mode3 = 0b011,
    Mode4 = 0b100,
    Mode5 = 0b101,
    Prohibited,
}

impl fmt::Display for LcdMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LcdMode::Mode0 => write!(f, "0"),
            LcdMode::Mode1 => write!(f, "1"),
            LcdMode::Mode2 => write!(f, "2"),
            LcdMode::Mode3 => write!(f, "3"),
            LcdMode::Mode4 => write!(f, "4"),
            LcdMode::Mode5 => write!(f, "5"),
            LcdMode::Prohibited => write!(f, "prohibited"),
        }
    }
}

assert_eq_size!(DisplayStatus, u16);

#[bitfield]
#[repr(u16)]
#[derive(Debug, Copy, Clone, Default)]
pub struct DisplayStatus {
    pub vblank_flag: bool,
    pub hblank_flag: bool,
    pub vcount_flag: bool,
    pub vblank_irq_enable: bool,
    pub hblank_irq_enable: bool,
    pub vcount_irq_enable: bool,
    #[skip]
    _reserved: B2,
    pub vcount_setting: u8,
}

impl GpuMemoryMappedIO for DisplayStatus {
    #[inline]
    fn write(&mut self, value: u16) {
        // *self = value.into()
        self.set_vblank_irq_enable((value >> 3) & 1 != 0);
        self.set_hblank_irq_enable((value >> 4) & 1 != 0);
        self.set_vcount_irq_enable((value >> 5) & 1 != 0);
        self.set_vcount_setting(usize::from((value >> 8) & 0xff) as u8);
    }

    #[inline]
    fn read(&self) -> u16 {
        u16::from(*self)
    }
}

#[derive(Debug, Copy, Clone)]
pub struct ObjBufferEntry {
    pub(crate) window: bool,
    pub(crate) alpha: bool,
    pub(crate) color: Rgb15,
    pub(crate) priority: u16,
}

impl Default for ObjBufferEntry {
    fn default() -> ObjBufferEntry {
        ObjBufferEntry {
            window: false,
            alpha: false,
            color: Rgb15::TRANSPARENT,
            priority: 4,
        }
    }
}

bitfield::bitfield! {
    #[repr(transparent)]
    #[derive(Copy, Clone, Default, PartialEq)]
    pub struct Rgb15(u16);
    impl Debug;
    pub r, set_r: 4, 0;
    pub g, set_g: 9, 5;
    pub b, set_b: 14, 10;
}

impl Rgb15 {
    pub const BLACK: Rgb15 = Rgb15(0);
    pub const WHITE: Rgb15 = Rgb15(0x7fff);
    pub const TRANSPARENT: Rgb15 = Rgb15(0x8000);

    pub fn to_rgb24(&self) -> u32 {
        ((self.r() as u32) << 19) | ((self.g() as u32) << 11) | ((self.b() as u32) << 3)
    }

    pub fn from_rgb(r: u16, g: u16, b: u16) -> Rgb15 {
        let mut c = Rgb15(0);
        c.set_r(r);
        c.set_g(g);
        c.set_b(b);
        c
    }

    pub fn get_rgb(&self) -> (u16, u16, u16) {
        (self.r(), self.g(), self.b())
    }

    pub fn is_transparent(&self) -> bool {
        self.0 == 0x8000
    }
}

#[derive(Debug, Default, Copy, Clone)]
pub struct BgAffine {
    pub pa: i16, // dx
    pub pb: i16, // dmx
    pub pc: i16, // dy
    pub pd: i16, // dmy
    pub x: i32,
    pub y: i32,
    pub internal_x: i32,
    pub internal_y: i32,
}

#[derive(Debug, Default, Clone)]
pub struct BgControl {
    pub priority: u16,
    pub character_base_block: u16,
    pub screen_base_block: u16,
    pub mosaic: bool,
    pub palette256: bool,
    pub affine_wraparound: bool,
    pub size: u8,
}

impl GpuMemoryMappedIO for BgControl {
    fn write(&mut self, value: u16) {
        self.priority = (value >> 0) & 0b11;
        self.character_base_block = (value >> 2) & 0b11;
        self.mosaic = (value >> 6) & 1 != 0;
        self.palette256 = (value >> 7) & 1 != 0;
        self.screen_base_block = (value >> 8) & 0b11111;
        self.affine_wraparound = (value >> 13) & 1 != 0;
        self.size = ((value >> 14) & 0b11) as u8;
    }

    fn read(&self) -> u16 {
        self.priority
            | self.character_base_block << 2
            | u16::from(self.mosaic) << 6
            | u16::from(self.palette256) << 7
            | self.screen_base_block << 8
            | u16::from(self.affine_wraparound) << 13
            | u16::from(self.size) << 14
    }
}

bitflags::bitflags! {
    #[derive(Default)]
    pub struct BlendFlags: u16 {
        const BG0 = 0b00000001;
        const BG1 = 0b00000010;
        const BG2 = 0b00000100;
        const BG3 = 0b00001000;
        const OBJ = 0b00010000;
        const BACKDROP  = 0b00100000; // BACKDROP
    }
}

impl BlendFlags {
    const BG_LAYER_FLAG: [BlendFlags; 4] = [
        BlendFlags::BG0,
        BlendFlags::BG1,
        BlendFlags::BG2,
        BlendFlags::BG3,
    ];
    #[inline]
    pub fn from_bg(bg: usize) -> BlendFlags {
        Self::BG_LAYER_FLAG[bg]
    }
    #[inline]
    pub fn obj_enabled(&self) -> bool {
        self.contains(BlendFlags::OBJ)
    }
    #[inline]
    pub fn contains_render_layer(&self, layer: &RenderLayer) -> bool {
        let layer_flags = BlendFlags::from_bits_truncate(layer.kind as u16);
        self.contains(layer_flags)
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum BlendMode {
    BldNone = 0b00,
    BldAlpha = 0b01,
    BldWhite = 0b10,
    BldBlack = 0b11,
}

impl Default for BlendMode {
    fn default() -> Self {
        BlendMode::BldNone
    }
}

#[derive(Debug, Default, Copy, Clone)]
pub struct BlendControl {
    pub target1: BlendFlags,
    pub target2: BlendFlags,
    pub mode: BlendMode,
}

impl GpuMemoryMappedIO for BlendControl {
    #[inline]
    fn write(&mut self, value: u16) {
        self.target1 = BlendFlags::from_bits_truncate((value >> 0) & 0x3f);
        self.target2 = BlendFlags::from_bits_truncate((value >> 8) & 0x3f);
        self.mode = BlendMode::from_u16((value >> 6) & 0b11).unwrap_or_else(|| unreachable!());
    }

    #[inline]
    fn read(&self) -> u16 {
        (self.target1.bits() << 0) | (self.mode as u16) << 6 | (self.target2.bits() << 8)
    }
}
impl BlendMode {
    pub fn from_u16(value: u16) -> Option<Self> {
        let ret = match value {
            0 => Self::BldNone,
            1 => Self::BldAlpha,
            2 => Self::BldWhite,
            3 => Self::BldBlack,
            _ => return None,
        };
        Some(ret)
    }
}

#[derive(Debug, Ord, Eq, PartialOrd, PartialEq, Clone, Copy)]
pub enum RenderLayerKind {
    Backdrop = 0b00100000,
    Background3 = 0b00001000,
    Background2 = 0b00000100,
    Background1 = 0b00000010,
    Background0 = 0b00000001,
    Objects = 0b00010000,
}

impl RenderLayerKind {
    pub fn get_blend_flag(&self) -> BlendFlags {
        match self {
            RenderLayerKind::Background0 => BlendFlags::BG0,
            RenderLayerKind::Background1 => BlendFlags::BG1,
            RenderLayerKind::Background2 => BlendFlags::BG2,
            RenderLayerKind::Background3 => BlendFlags::BG3,
            RenderLayerKind::Objects => BlendFlags::OBJ,
            RenderLayerKind::Backdrop => BlendFlags::BACKDROP,
        }
    }

    pub fn from_usize(val: usize) -> Option<Self> {
        let ret = match val {
            0b00100000 => Self::Backdrop,
            0b00001000 => Self::Background3,
            0b00000100 => Self::Background2,
            0b00000010 => Self::Background1,
            0b00000001 => Self::Background0,
            0b00010000 => Self::Objects,
            _ => return None,
        };
        Some(ret)
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct RenderLayer {
    pub kind: RenderLayerKind,
    pub priority: u16,
    pub pixel: Rgb15,
}

impl RenderLayer {
    pub fn background(bg: usize, pixel: Rgb15, priority: u16) -> RenderLayer {
        RenderLayer {
            kind: RenderLayerKind::from_usize(1 << bg).unwrap(),
            pixel,
            priority,
        }
    }

    pub fn objects(pixel: Rgb15, priority: u16) -> RenderLayer {
        RenderLayer {
            kind: RenderLayerKind::Objects,
            pixel,
            priority,
        }
    }

    pub fn backdrop(pixel: Rgb15) -> RenderLayer {
        RenderLayer {
            kind: RenderLayerKind::Backdrop,
            pixel,
            priority: 4,
        }
    }

    pub(super) fn is_object(&self) -> bool {
        self.kind == RenderLayerKind::Objects
    }
}

#[bitfield]
#[repr(u16)]
#[derive(Debug, Default, Clone, Copy)]
pub struct RegMosaic {
    bg_hsize: B4,
    bg_vsize: B4,
    obj_hsize: B4,
    obj_vsize: B4,
}

#[derive(Debug, Default, Copy, Clone)]
pub struct BlendAlpha {
    pub eva: u16,
    pub evb: u16,
}

impl GpuMemoryMappedIO for BlendAlpha {
    #[inline]
    fn write(&mut self, value: u16) {
        self.eva = value & 0x1f;
        self.evb = (value >> 8) & 0x1f;
    }

    #[inline]
    fn read(&self) -> u16 {
        self.eva | self.evb << 8
    }
}
