use fluorite_arm::registers::CpuState;
use fluorite_common::BitIndex;
use fluorite_gba::gba::Gba;
use fluorite_gba::keypad::{Keys, KEYINPUT_ALL_RELEASED};
use fluorite_gba::sysbus::Bus;
use fluorite_gba::VideoInterface;
use raylib::prelude::*;

use crate::consts::*;
use crate::utils::{DrawExt, RectExt};

macro_rules! bfe {
    ($value:expr, $offset:expr, $size:expr) => {
        ((($value) >> ($offset)) & ((1 << ($size)) - 1))
    };
}

pub struct EmulatorState {
    lcd: RenderTexture2D,
    keys: u16,

    // emulator state
    scroll: Vector2,
    last_rect: Rectangle,
    panel_mode: PanelMode,
    pub fps: u32,
    pub run_state: i32,
}

impl EmulatorState {
    pub fn new(lcd: RenderTexture2D) -> Self {
        Self {
            lcd,
            keys: KEYINPUT_ALL_RELEASED,
            scroll: Vector2::default(),
            last_rect: Rectangle::default(),
            panel_mode: PanelMode::Cpu,
            fps: 0,
            run_state: 1,
        }
    }

    pub fn reset(&mut self) {
        self.lcd.update_texture(&[0; 240 * 160 * 4]);
    }

    pub fn poll_keys(&mut self, rl: &RaylibHandle) {
        let mut keyinput = KEYINPUT_ALL_RELEASED;

        keyinput.set_bit(Keys::Up as usize, !rl.is_key_down(KeyboardKey::KEY_UP));
        keyinput.set_bit(Keys::Down as usize, !rl.is_key_down(KeyboardKey::KEY_DOWN));
        keyinput.set_bit(Keys::Left as usize, !rl.is_key_down(KeyboardKey::KEY_LEFT));
        keyinput.set_bit(
            Keys::Right as usize,
            !rl.is_key_down(KeyboardKey::KEY_RIGHT),
        );
        keyinput.set_bit(Keys::ButtonB as usize, !rl.is_key_down(KeyboardKey::KEY_Z));
        keyinput.set_bit(Keys::ButtonA as usize, !rl.is_key_down(KeyboardKey::KEY_X));
        keyinput.set_bit(
            Keys::Start as usize,
            !rl.is_key_down(KeyboardKey::KEY_ENTER),
        );
        keyinput.set_bit(
            Keys::Select as usize,
            !rl.is_key_down(KeyboardKey::KEY_SPACE),
        );
        keyinput.set_bit(Keys::ButtonL as usize, !rl.is_key_down(KeyboardKey::KEY_A));
        keyinput.set_bit(Keys::ButtonR as usize, !rl.is_key_down(KeyboardKey::KEY_S));

        self.keys = keyinput;
    }

    pub fn draw_frame(
        &mut self,
        gba: &mut Gba<EmulatorState>,
        rl: &mut RaylibHandle,
        thread: &RaylibThread,
    ) {
        let screen_width = rl.get_screen_width() as f32;
        let screen_height = rl.get_screen_height() as f32;

        let mut d = rl.begin_drawing(thread);

        d.clear_background(Color::WHITE);

        let panel_width = 430.0;
        // let panel_height = 30 + GUI_PADDING;
        // let lcd_aspect = GBA_LCD_H / GBA_LCD_W;
        let lcd_rect = Rectangle::new(panel_width, 0.0, screen_width - panel_width, screen_height);

        // draw side panel
        let rect = Rectangle::new(0.0, 0.0, panel_width, screen_height);
        let rect_inside = rect.shave(GUI_PADDING);

        // draw emu state
        let mut rect_inside = self.draw_emu_state(&mut d, rect_inside);

        self.last_rect.x = 0.0;
        self.last_rect.y = 0.0;
        self.last_rect.width = if self.last_rect.height < rect_inside.height {
            rect_inside.width - 5.0
        } else {
            rect_inside.width - d.gui_get_style(GuiControl::LISTVIEW, 18) as f32 - 5.0
        };

        let (view, _view_scale) = d.gui_scroll_panel(rect_inside, self.last_rect, self.scroll);
        self.scroll = _view_scale;

        rect_inside.y += self.scroll.y;
        let starty = rect_inside.y;
        rect_inside.y += GUI_PADDING as f32;
        rect_inside.x += GUI_PADDING as f32;
        rect_inside.width = view.width - GUI_PADDING as f32 * 1.5;

        {
            let mut s = d.begin_scissor_mode(
                view.x as i32,
                view.y as i32,
                view.width as i32,
                view.height as i32,
            );

            match self.panel_mode {
                PanelMode::Cpu => {
                    // rect_inside = draw_debug_state(rect_inside, &emu, &gb_state);
                    // rect_inside = draw_cartridge_state(rect_inside, &gb_state.cart);
                    rect_inside = self.draw_arm7_state(&mut s, rect_inside, gba);
                    // rect_inside = draw_joypad_state(rect_inside, &emu.joy);
                }
                PanelMode::Io => {
                    rect_inside = self.draw_io_state(&mut s, rect_inside, gba);
                }
                PanelMode::Audio => {
                    // rect_inside = draw_audio_state(rect_inside, &gb_state);
                }
            }
            self.last_rect.width = view.width - GUI_PADDING as f32;
            self.last_rect.height = rect_inside.y - starty;
        }

        // draw lcd screen
        let tex = &self.lcd;
        tex.set_texture_filter(thread, TextureFilter::TEXTURE_FILTER_POINT);
        d.draw_texture_quad(
            tex,
            Vector2::new(1.0, 1.0),
            Vector2::new(0.0, 0.0),
            lcd_rect,
            Color::WHITE,
        );

        // d.draw_text(
        //     &format!("{:#?}", &gba.scheduler.events),
        //     500,
        //     25,
        //     12,
        //     Color::BLACK,
        // );
    }

    fn draw_arm7_state(
        &self,
        d: &mut impl RaylibDraw,
        rect: Rectangle,
        gba: &mut Gba<EmulatorState>,
    ) -> Rectangle {
        let arm = gba.arm_cpu();
        let mut inside_rect = rect.shave(GUI_PADDING);

        // Split registers into two rects horizontally
        {
            let mut in_rect = [Rectangle::default(); 2];
            let sections = ["Registers", "Banked Registers"];
            let orig_y = inside_rect.y;
            let mut x_off = 0.0;
            for r in &mut in_rect {
                *r = inside_rect;
                r.width = inside_rect.width / 2.0 - GUI_PADDING as f32 * 1.0 / 2.0;
                r.x += x_off;
                x_off += r.width + GUI_PADDING as f32;
            }
            let reg_names = [
                "R0", "R1", "R2", "R3", "R4", "R5", "R6", "R7", "R8", "R9", "R10", "R11", "R12",
                "R13", "R14", "R15(PC)", "CPSR", "N", "Z", "C", "V",
            ];
            let mut reg_vals = [0; 21];
            for (i, val) in reg_vals.iter_mut().enumerate().take(16) {
                *val = arm.get_reg(i);
            }

            reg_vals[16] = arm.get_cspr();
            reg_vals[17] = bfe!(reg_vals[16], 31, 1);
            reg_vals[18] = bfe!(reg_vals[16], 30, 1);
            reg_vals[19] = bfe!(reg_vals[16], 29, 1);
            reg_vals[20] = bfe!(reg_vals[16], 28, 1);

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
            let _banked_vals = [0; 20];
            // for(int i=0;i<5;++i) banked_vals[i]   = arm->registers[32+i];
            // for(int i=0;i<7;++i) banked_vals[5+i] = arm->registers[17+i];
            // for(int i=0;i<2;++i) banked_vals[12+i]= arm->registers[24+i];
            // for(int i=0;i<2;++i) banked_vals[14+i]= arm->registers[26+i];
            // for(int i=0;i<2;++i) banked_vals[16+i]= arm->registers[28+i];
            // for(int i=0;i<2;++i) banked_vals[18+i]= arm->registers[30+i];

            in_rect[0] = Self::draw_reg_state(d, in_rect[0], "Registers", reg_names, reg_vals);
            // in_rect[1] = draw_reg_state(in_rect[1], "Banked Registers", banked_regs, banked_vals);

            for r in &in_rect {
                if inside_rect.y < r.y {
                    inside_rect.y = r.y;
                }
            }
            for i in 0..2 {
                in_rect[i].height = inside_rect.y - orig_y - GUI_PADDING as f32;
                in_rect[i].y = orig_y;
                d.gui_group_box(in_rect[i], Some(&rstr!("{}", sections[i])));
            }
            inside_rect.height -= inside_rect.y - orig_y;
        }

        inside_rect = self.draw_instructions(d, inside_rect, gba);

        let (state_rect, adv_rect) = rect.chop((inside_rect.y - rect.y) as i32, GUI_PADDING);

        d.gui_group_box(state_rect, Some(rstr!("ARM7 State")));
        adv_rect
    }

    fn draw_instructions(
        &self,
        d: &mut impl RaylibDraw,
        rect: Rectangle,
        gba: &mut Gba<EmulatorState>,
    ) -> Rectangle {
        let mut inside_rect = rect.shave(GUI_PADDING);
        // let mut widget_rect;
        let arm = gba.arm_cpu_mut();
        let pc = arm.get_reg(15);

        // TODO: disassemble THUMB

        let state = arm.get_cpu_state();
        let mut disasm = String::with_capacity(64);

        for i in -6..5i32 {
            let (mut widget_rect, new_inside_rect) =
                inside_rect.chop(GUI_LABEL_HEIGHT, GUI_PADDING + 5);

            let pc_render = i * (if state == CpuState::THUMB { 2 } else { 4 }) + pc as i32
                - if state == CpuState::THUMB { 4 } else { 8 };

            if pc_render < 0 {
                widget_rect.x += 80.0;
                d.gui_label(widget_rect, Some(rstr!("INVALID")));
            } else {
                if i == 0 {
                    d.gui_label(widget_rect, Some(rstr!("PC->")));
                }
                widget_rect.x += 30.0;
                d.gui_label(widget_rect, Some(&rstr!("{:08X}", pc_render)));
                widget_rect.x += 80.0;

                let opcode = arm.get_instructionge(pc_render as u32, &mut disasm);
                d.gui_label(widget_rect, Some(&rstr!("{}", disasm)));
                disasm.clear();

                widget_rect.x += 150.0;
                d.gui_label(
                    widget_rect,
                    Some(&match state {
                        CpuState::ARM => rstr!("{:08X}", opcode),
                        CpuState::THUMB => rstr!("{:04X}", opcode),
                    }),
                );
                widget_rect.x += 50.0;
            }

            inside_rect = new_inside_rect
        }

        let (state_rect, adv_rect) = rect.chop((inside_rect.y - rect.y) as i32, GUI_PADDING);
        d.gui_group_box(state_rect, Some(&rstr!("Instructions [{}]", state)));
        adv_rect
    }

    fn draw_io_state(
        &self,
        d: &mut impl RaylibDraw,
        rect: Rectangle,
        gba: &mut Gba<EmulatorState>,
    ) -> Rectangle {
        let mut rect = rect;

        for reg in IO_REGS {
            let mut r = rect.shave(GUI_PADDING);
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
                        r = d.draw_label(r, &rstr!("[{}:{}]:", start, start + size - 1));
                    } else {
                        r = d.draw_label(r, &rstr!("{}:", start));
                    }

                    r2.x += 30.0;
                    d.draw_label(r2, &rstr!("{}", field_data));
                    r2.x += 25.0;
                    d.draw_label(r2, &rstr!("{}", bit.name));
                }
            }

            let (state_rect, adv_rect) = rect.chop(r.y as i32 - rect.y as i32, GUI_PADDING);
            d.gui_group_box(
                state_rect,
                Some(&rstr!("{}({:X}): {:04X}", reg.name, addr, data)),
            );
            rect = adv_rect;
        }

        rect
    }

    fn draw_emu_state(&mut self, d: &mut impl RaylibDraw, rect: Rectangle) -> Rectangle {
        let (mut widget_rect, inside_rect) =
            rect.shave(GUI_PADDING).chop(GUI_ROW_HEIGHT, GUI_PADDING);

        widget_rect.width =
            widget_rect.width / 4.0 - d.gui_get_style(GuiControl::TOGGLE, 16) as f32;
        // todo link this with the emulator
        self.run_state = d.gui_toggle_group(
            widget_rect,
            Some(rstr!("#74#Reset;#132#Pause;#131#Run;#134#Step")),
            self.run_state,
        );

        let (mut widget_rect, inside_rect) = inside_rect.chop(GUI_ROW_HEIGHT, GUI_PADDING);

        d.gui_label(widget_rect, Some(rstr!("Panel Mode")));
        widget_rect.width =
            widget_rect.width / 3.0 - d.gui_get_style(GuiControl::TOGGLE, 16) as f32 * 2.0 / 3.0;

        self.panel_mode = BUTTON_STATES[d.gui_toggle_group(
            widget_rect,
            Some(rstr!("CPU;IO Regs;Audio")),
            self.panel_mode as i32,
        ) as usize];

        let (state_rect, adv_rect) = rect.chop((inside_rect.y - rect.y) as i32, GUI_PADDING);
        // TODO: get avg frame time from emulator state
        d.gui_group_box(
            state_rect,
            Some(&rstr!("Emulator State [FPS: {}]", self.fps)),
        );

        adv_rect
    }

    fn draw_reg_state<const N: usize>(
        d: &mut impl RaylibDraw,
        rect: Rectangle,
        _group_name: &str,
        names: [&str; N],
        values: [u32; N],
    ) -> Rectangle {
        let mut inside_rect = rect.shave(GUI_PADDING);

        for i in 0..N {
            let (mut widget_rect, new_inside_rect) =
                inside_rect.chop(GUI_LABEL_HEIGHT, GUI_PADDING + 5);

            d.gui_label(widget_rect, Some(&rstr!("{}", names[i])));
            let w = (new_inside_rect.width - GUI_PADDING as f32 * 2.0) / 3.0;
            widget_rect.x += w;
            d.gui_label(widget_rect, Some(&rstr!("0x{:X}", values[i])));

            widget_rect.x += w + GUI_PADDING as f32 * 2.0;
            d.gui_label(widget_rect, Some(&rstr!("{}", values[i])));

            inside_rect = new_inside_rect;
        }

        let (_, adv_rect) = rect.chop((inside_rect.y - rect.y) as i32, GUI_PADDING);

        adv_rect
    }
}

impl VideoInterface for EmulatorState {
    fn render(&mut self, buffer: &[u8]) {
        self.lcd.update_texture(buffer);
    }

    fn poll(&mut self) -> u16 {
        self.keys
    }
}
