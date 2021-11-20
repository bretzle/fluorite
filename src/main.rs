use fluorite_gba::gba::Gba;
use fluorite_gba::sysbus::Bus;
use fluorite_gba::VideoInterface;
use raylib::prelude::*;
use std::ffi::CStr;
use std::fmt::Write;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use std::{cell::RefCell, rc::Rc};

use crate::fps::FpsCounter;

mod fps;

static BIOS: &[u8] = include_bytes!("../roms/gba_bios.bin");

macro_rules! bfe {
    ($value:expr, $offset:expr, $size:expr) => {
        ((($value) >> ($offset)) & ((1 << ($size)) - 1))
    };
}

struct Screen(RenderTexture2D);

impl Screen {
    pub fn get_tex(&self) -> &RenderTexture2D {
        &self.0
    }
}

impl VideoInterface for Screen {
    fn render(&mut self, buffer: &[u8]) {
        self.0.update_texture(buffer);
    }
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let (rom, name) = {
        let mut file_path = PathBuf::new();
        match std::env::args().nth(1) {
            Some(s) => file_path.push(s),
            None => file_path.push("roms/beeg.bin"),
        };
        let mut file = File::open(&file_path)?;
        let mut buf = vec![];
        file.read_to_end(&mut buf)?;
        (
            buf,
            file_path.file_stem().unwrap().to_string_lossy().to_string(),
        )
    };

    let (mut rl, thread) = raylib::init()
        .size(430 + (240 * 4), 160 * 4)
        .title("Fluorite")
        .vsync()
        .build();

    rl.set_exit_key(None);

    println!("--------------");

    let tex = rl.load_render_texture(&thread, 240, 160).unwrap();

    let device = Rc::new(RefCell::new(Screen(tex)));
    let mut counter = FpsCounter::default();
    let mut gba = Gba::new(device.clone(), BIOS, &rom);

    gba.skip_bios();
    let mut title = "".to_string();

    let mut emu = EmulatorState::new();

    while !rl.window_should_close() {
        gba.frame();

        if let Some(real) = counter.tick() {
            emu.fps = real as f64;
            let time = gba.render_time();
            let fps = 1.0 / time.as_secs_f64();
            title.clear();
            write!(
                &mut title,
                "{} | Render: {} ({:?})",
                name,
                fps.round(),
                time
            )?;
            rl.set_window_title(&thread, &title);
        }

        draw_frame(&mut emu, &mut gba, &mut rl, &thread, &device);
    }

    Ok(())
}

const GUI_PADDING: i32 = 10;
const GUI_ROW_HEIGHT: i32 = 30;
const GUI_LABEL_HEIGHT: i32 = 0;
const GUI_LABEL_PADDING: i32 = 5;

const GBA_LCD_W: f32 = 240.0;
const GBA_LCD_H: f32 = 160.0;

fn draw_frame(
    emu: &mut EmulatorState,
    gba: &mut Gba<Screen>,
    rl: &mut RaylibHandle,
    thread: &RaylibThread,
    device: &Rc<RefCell<Screen>>,
) {
    let screen_width = rl.get_screen_width() as f32;
    let screen_height = rl.get_screen_height() as f32;

    let mut d = rl.begin_drawing(&thread);

    d.clear_background(Color::WHITE);

    let panel_width = 430.0;
    let panel_height = 30 + GUI_PADDING;
    let lcd_aspect = GBA_LCD_H / GBA_LCD_W;
    let lcd_rect = Rectangle::new(panel_width, 0.0, screen_width - panel_width, screen_height);

    // draw side panel
    let rect = Rectangle::new(0.0, 0.0, panel_width, screen_height);
    let rect_inside: Rectangle = inside_rect_after_padding(rect, GUI_PADDING);

    // draw emu state
    let mut rect_inside = draw_emu_state(&mut d, rect_inside, emu, gba);

    emu.last_rect.x = 0.0;
    emu.last_rect.y = 0.0;
    emu.last_rect.width = if emu.last_rect.height < rect_inside.height {
        rect_inside.width - 5.0
    } else {
        rect_inside.width - d.gui_get_style(GuiControl::LISTVIEW, 18) as f32 - 5.0
    };

    let (view, view_scale) = d.gui_scroll_panel(rect_inside, emu.last_rect, emu.scroll);

    rect_inside.y += emu.scroll.y;
    let starty = rect_inside.y;
    rect_inside.y += GUI_PADDING as f32;
    rect_inside.x += GUI_PADDING as f32;
    rect_inside.width = view.width - GUI_PADDING as f32 * 1.5;

    match emu.panel_mode {
        PanelMode::CPU => {
            // rect_inside = draw_debug_state(rect_inside, &emu, &gb_state);
            // rect_inside = draw_cartridge_state(rect_inside, &gb_state.cart);
            rect_inside = draw_arm7_state(&mut d, rect_inside, gba);
            // rect_inside = draw_joypad_state(rect_inside, &emu.joy);
        }
        PanelMode::IO => {
            rect_inside = draw_io_state(&mut d, rect_inside, gba);
        }
        PanelMode::AUDIO => {
            // rect_inside = draw_audio_state(rect_inside, &gb_state);
        }
    }
    emu.last_rect.width = view.width - GUI_PADDING as f32;
    emu.last_rect.height = rect_inside.y - starty;

    // draw lcd screen
    let screen = device.borrow();
    let tex = screen.get_tex();
    tex.set_texture_filter(
        thread,
        if lcd_rect.width < (tex.width() * 2) as f32 {
            TextureFilter::TEXTURE_FILTER_BILINEAR
        } else {
            TextureFilter::TEXTURE_FILTER_POINT
        },
    );
    d.draw_texture_quad(
        tex,
        Vector2::new(1.0, 1.0),
        Vector2::new(0.0, 0.0),
        lcd_rect,
        Color::WHITE,
    );
}

fn draw_arm7_state(d: &mut RaylibDrawHandle, rect: Rectangle, gba: &mut Gba<Screen>) -> Rectangle {
    let arm = gba.arm_cpu();
    let mut inside_rect = inside_rect_after_padding(rect, GUI_PADDING);

    // Split registers into two rects horizontally
    {
        let mut in_rect = [Rectangle::default(); 2];
        let sections = ["Registers", "Banked Registers"];
        let orig_y = inside_rect.y;
        let mut x_off = 0.0;
        for i in 0..2 {
            in_rect[i] = inside_rect;
            in_rect[i].width = inside_rect.width / 2.0 - GUI_PADDING as f32 * 1.0 / 2.0;
            in_rect[i].x += x_off;
            x_off += in_rect[i].width + GUI_PADDING as f32;
        }
        let reg_names = [
            "R0", "R1", "R2", "R3", "R4", "R5", "R6", "R7", "R8", "R9", "R10", "R11", "R12", "R13",
            "R14", "R15(PC)", "CPSR", "N", "Z", "C", "V",
        ];
        let mut reg_vals = [0; 21];
        for i in 0..16 {
            reg_vals[i] = arm.get_reg(i);
        }

        reg_vals[16] = arm.get_cspr();
        reg_vals[17] = bfe!(reg_vals[16], 31, 1);
        reg_vals[18] = bfe!(reg_vals[16], 30, 1);
        reg_vals[19] = bfe!(reg_vals[16], 29, 1);
        reg_vals[20] = bfe!(reg_vals[16], 28, 1);

        let banked_regs = [
            "SPSRfiq", "SPSRirq", "SPSRsvc", "SPSRabt", "SPSRund", "R8fiq", "R9fiq", "R10fiq",
            "R11fiq", "R12fiq", "R13fiq", "R14fiq", "R13irq", "R14irq", "R13svc", "R14svc",
            "R13abt", "R14abt", "R13und", "R14und",
        ];
        /*
        Banked Reg Table
        17-23: R8_fiq-R14_fiq
        24-25: R13_irq-R14_irq
        26-27: R13_svc-R14_svc
        28-29: R13_abt-R14_abt
        30-31: R13_und-R14_und
        32: SPSR_fiq
        33: SPSR_irq
        34: SPSR_svc
        35: SPSR_abt
        36: SPSR_und
        */
        let banked_vals = [0; 20];
        // for(int i=0;i<5;++i) banked_vals[i]   = arm->registers[32+i];
        // for(int i=0;i<7;++i) banked_vals[5+i] = arm->registers[17+i];
        // for(int i=0;i<2;++i) banked_vals[12+i]= arm->registers[24+i];
        // for(int i=0;i<2;++i) banked_vals[14+i]= arm->registers[26+i];
        // for(int i=0;i<2;++i) banked_vals[16+i]= arm->registers[28+i];
        // for(int i=0;i<2;++i) banked_vals[18+i]= arm->registers[30+i];

        in_rect[0] = draw_reg_state(d, in_rect[0], "Registers", reg_names, reg_vals);
        // in_rect[1] = draw_reg_state(in_rect[1], "Banked Registers", banked_regs, banked_vals);

        for i in 0..2 {
            if inside_rect.y < in_rect[i].y {
                inside_rect.y = in_rect[i].y;
            }
        }
        for i in 0..2 {
            in_rect[i].height = inside_rect.y - orig_y - GUI_PADDING as f32;
            in_rect[i].y = orig_y;
            d.gui_group_box(in_rect[i], Some(&rstr!("{}", sections[i])));
        }
        inside_rect.height -= inside_rect.y - orig_y;
    }

    //   inside_rect = gba_draw_instructions(inside_rect, gba);

    let mut state_rect = Rectangle::default();
    let mut adv_rect = Rectangle::default();
    vertical_adv(
        rect,
        (inside_rect.y - rect.y) as i32,
        GUI_PADDING,
        &mut state_rect,
        &mut adv_rect,
    );
    d.gui_group_box(state_rect, Some(rstr!("ARM7 State")));
    return adv_rect;
}

fn draw_reg_state(
    d: &mut RaylibDrawHandle,
    rect: Rectangle,
    group_name: &str,
    names: [&str; 21],
    values: [u32; 21],
) -> Rectangle {
    let mut inside_rect = inside_rect_after_padding(rect, GUI_PADDING);
    let mut widget_rect = Rectangle::default();
    for i in 0..21 {
        vertical_adv(
            inside_rect,
            GUI_LABEL_HEIGHT,
            GUI_PADDING + 5,
            &mut widget_rect,
            &mut inside_rect,
        );
        d.gui_label(widget_rect, Some(&rstr!("{}", names[i])));
        let w = (inside_rect.width - GUI_PADDING as f32 * 2.0) / 3.0;
        widget_rect.x += w;
        d.gui_label(widget_rect, Some(&rstr!("0x{:X}", values[i])));

        widget_rect.x += w + GUI_PADDING as f32 * 2.0;
        d.gui_label(widget_rect, Some(&rstr!("{}", values[i])));
    }

    let mut state_rect = Rectangle::default();
    let mut adv_rect = Rectangle::default();

    vertical_adv(
        rect,
        (inside_rect.y - rect.y) as i32,
        GUI_PADDING,
        &mut state_rect,
        &mut adv_rect,
    );

    adv_rect
}

struct MmioRegBit {
    start: u8,
    size: u8,
    name: &'static str,
}

struct MmioReg {
    addr: u32,
    name: &'static str,
    bits: [MmioRegBit; 16],
}

#[rustfmt::skip]
const IO_REGS: [MmioReg; 2] = [
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

fn draw_io_state(d: &mut RaylibDrawHandle, rect: Rectangle, gba: &mut Gba<Screen>) -> Rectangle {
    let mut rect = rect;

    for reg in IO_REGS {
        let mut r = inside_rect_after_padding(rect, GUI_PADDING);
        let addr = reg.addr;
        let data = gba.sysbus.read_16(addr);
        // let mut has_fields = false;
        for bit in reg.bits {
            let start = bit.start;
            let size = bit.size;
            if size > 0 {
                let field_data = bfe!(data, start, size);
                // has_fields = true;
                let mut r2 = r;
                if size > 1 {
                    r = draw_label(d, r, &rstr!("[{}:{}]:", start, start + size - 1));
                } else {
                    r = draw_label(d, r, &rstr!("{}:", start));
                }

                r2.x += 30.0;
                draw_label(d, r2, &rstr!("{}", field_data));
                r2.x += 25.0;
                draw_label(d, r2, &rstr!("{}", bit.name));
            }
        }
        let mut state_rect = Rectangle::default();
        let mut adv_rect = Rectangle::default();
        vertical_adv(
            rect,
            r.y as i32 - rect.y as i32,
            GUI_PADDING,
            &mut state_rect,
            &mut adv_rect,
        );
        d.gui_group_box(
            state_rect,
            Some(&rstr!("{}({:X}): {:04X}", reg.name, addr, data)),
        );
        rect = adv_rect;
    }

    rect
}

fn draw_emu_state(
    d: &mut RaylibDrawHandle,
    rect: Rectangle,
    emu: &mut EmulatorState,
    gba: &mut Gba<Screen>,
) -> Rectangle {
    let mut inside_rect = inside_rect_after_padding(rect, GUI_PADDING);
    let mut widget_rect = Rectangle::default();

    vertical_adv(
        inside_rect,
        GUI_ROW_HEIGHT,
        GUI_PADDING,
        &mut widget_rect,
        &mut inside_rect,
    );
    widget_rect.width = widget_rect.width / 4.0 - d.gui_get_style(GuiControl::TOGGLE, 16) as f32;
    // todo link this with the emulator
    let run_state = d.gui_toggle_group(
        widget_rect,
        Some(rstr!("#74#Reset;#132#Pause;#131#Run;#134#Step")),
        -1,
    );

    vertical_adv(
        inside_rect,
        GUI_ROW_HEIGHT,
        GUI_PADDING,
        &mut widget_rect,
        &mut inside_rect,
    );

    let mut state_rect = Rectangle::default();
    let mut adv_rect = Rectangle::default();

    d.gui_label(widget_rect, Some(rstr!("Panel Mode")));
    widget_rect.width =
        widget_rect.width / 3.0 - d.gui_get_style(GuiControl::TOGGLE, 16) as f32 * 2.0 / 3.0;
    let button_state = [PanelMode::CPU, PanelMode::IO, PanelMode::AUDIO];

    emu.panel_mode = button_state[d.gui_toggle_group(
        widget_rect,
        Some(rstr!("CPU;IO Regs;Audio")),
        emu.panel_mode as i32,
    ) as usize];

    vertical_adv(
        rect,
        (inside_rect.y - rect.y) as i32,
        GUI_PADDING,
        &mut state_rect,
        &mut adv_rect,
    );
    // TODO: get avg frame time from emulator state
    d.gui_group_box(
        state_rect,
        Some(&rstr!("Emulator State [FPS: {}]", emu.fps)),
    );

    adv_rect
}

fn vertical_adv(
    outside_rect: Rectangle,
    advance: i32,
    y_padding: i32,
    rect_top: &mut Rectangle,
    rect_bottom: &mut Rectangle,
) {
    *rect_top = outside_rect;
    rect_top.height = advance as f32;
    *rect_bottom = outside_rect;
    rect_bottom.y += (advance + y_padding) as f32;
    rect_bottom.height -= (advance + y_padding) as f32;
}

fn inside_rect_after_padding(rect: Rectangle, gui_padding: i32) -> Rectangle {
    let mut inside = rect;
    let padding = gui_padding as f32;
    inside.x += padding;
    inside.y += padding;
    inside.width -= padding * 2.0;
    inside.height -= padding * 2.0;
    inside
}

fn draw_label(d: &mut RaylibDrawHandle, mut layout_rect: Rectangle, label: &CStr) -> Rectangle {
    let mut widget_rect = Rectangle::default();
    vertical_adv(
        layout_rect,
        GUI_LABEL_HEIGHT,
        GUI_PADDING,
        &mut widget_rect,
        &mut layout_rect,
    );
    d.gui_label(widget_rect, Some(label));
    return layout_rect;
}

struct EmulatorState {
    scroll: Vector2,
    last_rect: Rectangle,
    panel_mode: PanelMode,
    fps: f64,
}

#[repr(i32)]
#[derive(Debug, Clone, Copy)]
enum PanelMode {
    CPU = 0,
    IO = 1,
    AUDIO = 2,
}

impl EmulatorState {
    pub fn new() -> Self {
        Self {
            scroll: Vector2::default(),
            last_rect: Rectangle::default(),
            panel_mode: PanelMode::CPU,
            fps: 60.0,
        }
    }
}
