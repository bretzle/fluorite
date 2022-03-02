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
    lcd: RenderTexture,
    keys: u16,

    // audio: RaylibAudio,
    // audio_stream: AudioStream,

    // emulator state
    scroll: Vector2,
    last_rect: Rectangle,
    panel_mode: PanelMode,
    pub fps: u32,
    pub run_state: usize,
}

impl EmulatorState {
    pub fn new(lcd: RenderTexture) -> Self {
        // let mut audio = RaylibAudio::init_audio_device();
        // let mut audio_stream = AudioStream::init_audio_stream(thread, 44100, 16, 2);

        // audio.play_audio_stream(&mut audio_stream);

        Self {
            lcd,
            keys: KEYINPUT_ALL_RELEASED,
            // audio,
            // audio_stream,
            scroll: Vector2::default(),
            last_rect: Rectangle::default(),
            panel_mode: PanelMode::Cpu,
            fps: 0,
            run_state: 1,
        }
    }

    pub fn reset(&mut self) {
        // self.lcd.update_texture(&[0; 240 * 160 * 4]);
    }

    pub fn poll_keys(&mut self, rl: &Raylib) {
        let mut keyinput = KEYINPUT_ALL_RELEASED;

        keyinput.set_bit(Keys::Up as usize, !rl.IsKeyDown(Key::Up));
        keyinput.set_bit(Keys::Down as usize, !rl.IsKeyDown(Key::Down));
        keyinput.set_bit(Keys::Left as usize, !rl.IsKeyDown(Key::Left));
        keyinput.set_bit(Keys::Right as usize, !rl.IsKeyDown(Key::Right));
        keyinput.set_bit(Keys::ButtonB as usize, !rl.IsKeyDown(Key::Z));
        keyinput.set_bit(Keys::ButtonA as usize, !rl.IsKeyDown(Key::X));
        keyinput.set_bit(Keys::Start as usize, !rl.IsKeyDown(Key::Enter));
        keyinput.set_bit(Keys::Select as usize, !rl.IsKeyDown(Key::Space));
        keyinput.set_bit(Keys::ButtonL as usize, !rl.IsKeyDown(Key::A));
        keyinput.set_bit(Keys::ButtonR as usize, !rl.IsKeyDown(Key::S));

        self.keys = keyinput;
    }

    pub fn draw_frame(&mut self, gba: &mut Gba<EmulatorState>, rl: &mut Raylib) {
        let screen_width = rl.GetScreenWidth() as f32;
        let screen_height = rl.GetScreenHeight() as f32;

        rl.begin_drawing();
        let d = rl;

        d.clear_background(Color::WHITE);

        let panel_width = 430.0;
        // let panel_height = 30 + GUI_PADDING;
        // let lcd_aspect = GBA_LCD_H / GBA_LCD_W;
        let lcd_rect = Rectangle::new(panel_width, 0.0, screen_width - panel_width, screen_height);

        // draw side panel
        let rect = Rectangle::new(0.0, 0.0, panel_width, screen_height);
        let rect_inside = rect.shave(GUI_PADDING);

        // draw emu state
        let mut rect_inside = self.draw_emu_state(d, rect_inside);

        self.last_rect.x = 0.0;
        self.last_rect.y = 0.0;
        self.last_rect.width = if self.last_rect.height < rect_inside.height {
            rect_inside.width - 5.0
        } else {
            rect_inside.width - d.get_style().listview.scrollbar_width as f32 - 5.0
        };

        let (view, _view_scale) = d.GuiScrollPanel(rect_inside, self.last_rect, self.scroll);
        self.scroll = _view_scale;

        rect_inside.y += self.scroll.y;
        let starty = rect_inside.y;
        rect_inside.y += GUI_PADDING as f32;
        rect_inside.x += GUI_PADDING as f32;
        rect_inside.width = view.width - GUI_PADDING as f32 * 1.5;

        {
            d.BeginScissorMode(
                view.x as i32,
                view.y as i32,
                view.width as i32,
                view.height as i32,
            );

            match self.panel_mode {
                PanelMode::Cpu => {
                    // rect_inside = draw_debug_state(rect_inside, &emu, &gb_state);
                    // rect_inside = draw_cartridge_state(rect_inside, &gb_state.cart);
                    rect_inside = self.draw_joypad_state(d, rect_inside, gba);
                    rect_inside = self.draw_arm7_state(d, rect_inside, gba);
                }
                PanelMode::Io => {
                    rect_inside = self.draw_io_state(d, rect_inside, gba);
                }
                PanelMode::Audio => {
                    rect_inside = self.draw_audio_state(d, rect_inside, gba);
                }
            }
            self.last_rect.width = view.width - GUI_PADDING as f32;
            self.last_rect.height = rect_inside.y - starty;

            d.EndScissorMode();
        }

        d.end_drawing();

        // draw lcd screen
        let tex = &self.lcd;
        tex.SetTextureFilter(TextureFilter::Point);
        d.DrawTextureQuad(
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
        d: &mut Raylib,
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
                d.GuiGroupBox(in_rect[i], sections[i]);
            }
            inside_rect.height -= inside_rect.y - orig_y;
        }

        inside_rect = self.draw_instructions(d, inside_rect, gba);

        let (state_rect, adv_rect) = rect.chop((inside_rect.y - rect.y) as i32, GUI_PADDING);

        d.GuiGroupBox(state_rect, "ARM7 State");
        adv_rect
    }

    fn draw_instructions(
        &self,
        d: &mut Raylib,
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
                d.GuiLabel(widget_rect, "INVALID");
            } else {
                if i == 0 {
                    d.GuiLabel(widget_rect, "PC->");
                }
                widget_rect.x += 30.0;
                d.GuiLabel(widget_rect, &format!("{:08X}", pc_render));
                widget_rect.x += 80.0;

                let opcode = arm.get_instructionge(pc_render as u32, &mut disasm);
                d.GuiLabel(widget_rect, disasm.as_str());
                disasm.clear();

                widget_rect.x += 150.0;
                d.GuiLabel(
                    widget_rect,
                    &match state {
                        CpuState::ARM => format!("{:08X}", opcode),
                        CpuState::THUMB => format!("{:04X}", opcode),
                    },
                );
                widget_rect.x += 50.0;
            }

            inside_rect = new_inside_rect
        }

        let (state_rect, adv_rect) = rect.chop((inside_rect.y - rect.y) as i32, GUI_PADDING);
        d.GuiGroupBox(state_rect, &format!("Instructions [{}]", state));
        adv_rect
    }

    fn draw_io_state(
        &self,
        d: &mut Raylib,
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
                if let Some(bit) = bit {
                    let start = bit.start;
                    let size = bit.size;
                    if size > 0 {
                        let field_data = bfe!(data, start, size);
                        // has_fields = true;
                        let mut r2 = r;
                        if size > 1 {
                            r = d.draw_label(r, &format!("[{}:{}]:", start, start + size - 1));
                        } else {
                            r = d.draw_label(r, &format!("{}:", start));
                        }

                        r2.x += 30.0;
                        d.draw_label(r2, &format!("{}", field_data));
                        r2.x += 25.0;
                        d.draw_label(r2, bit.name);
                    }
                }
            }

            let (state_rect, adv_rect) = rect.chop(r.y as i32 - rect.y as i32, GUI_PADDING);
            d.GuiGroupBox(
                state_rect,
                &format!("{}({:X}): {:04X}", reg.name, addr, data),
            );
            rect = adv_rect;
        }

        rect
    }

    fn draw_emu_state(&mut self, d: &mut Raylib, rect: Rectangle) -> Rectangle {
        let (mut widget_rect, inside_rect) =
            rect.shave(GUI_PADDING).chop(GUI_ROW_HEIGHT, GUI_PADDING);

        widget_rect.width = widget_rect.width / 4.0 - d.get_style().toggle.text_spacing as f32;
        // todo link this with the emulator
        self.run_state = d.GuiToggleGroup(
            widget_rect,
            &["#74#Reset", "132#Pause", "#131#Run", "#134#Step"],
            self.run_state,
        );

        let (mut widget_rect, inside_rect) = inside_rect.chop(GUI_ROW_HEIGHT, GUI_PADDING);

        d.GuiLabel(widget_rect, "Panel Mode");
        widget_rect.width =
            widget_rect.width / 3.0 - d.get_style().toggle.text_spacing as f32 * 2.0 / 3.0;

        self.panel_mode = BUTTON_STATES[d.GuiToggleGroup(
            widget_rect,
            &["CPU", "IO Regs", "Audio"],
            self.panel_mode as usize,
        ) as usize];

        let (state_rect, adv_rect) = rect.chop((inside_rect.y - rect.y) as i32, GUI_PADDING);
        // TODO: get avg frame time from emulator state
        d.GuiGroupBox(state_rect, &format!("Emulator State [FPS: {}]", self.fps));

        adv_rect
    }

    fn draw_reg_state<const N: usize>(
        d: &mut Raylib,
        rect: Rectangle,
        _group_name: &str,
        names: [&str; N],
        values: [u32; N],
    ) -> Rectangle {
        let mut inside_rect = rect.shave(GUI_PADDING);

        for i in 0..N {
            let (mut widget_rect, new_inside_rect) =
                inside_rect.chop(GUI_LABEL_HEIGHT, GUI_PADDING + 5);

            d.GuiLabel(widget_rect, names[i]);
            let w = (new_inside_rect.width - GUI_PADDING as f32 * 2.0) / 3.0;
            widget_rect.x += w;
            d.GuiLabel(widget_rect, &format!("0x{:X}", values[i]));

            widget_rect.x += w + GUI_PADDING as f32 * 2.0;
            d.GuiLabel(widget_rect, &format!("{}", values[i]));

            inside_rect = new_inside_rect;
        }

        let (_, adv_rect) = rect.chop((inside_rect.y - rect.y) as i32, GUI_PADDING);

        adv_rect
    }

    fn draw_joypad_state(
        &self,
        d: &mut Raylib,
        rect: Rectangle,
        gba: &mut Gba<EmulatorState>,
    ) -> Rectangle {
        let inside_rect = rect.shave(GUI_PADDING);
        let mut wr = inside_rect;
        wr.width = GUI_PADDING as f32;
        wr.height = GUI_PADDING as f32;

        let keys = gba.sysbus.read_16(0x04000130);

        let (widget_rect, inside_rect) = inside_rect.chop(GUI_LABEL_HEIGHT, GUI_PADDING);
        wr.y = widget_rect.y;
        d.GuiCheckBox(wr, "Up", !keys.bit(6));
        let (widget_rect, inside_rect) = inside_rect.chop(GUI_LABEL_HEIGHT, GUI_PADDING);
        wr.y = widget_rect.y;
        d.GuiCheckBox(wr, "Down", !keys.bit(7));
        let (widget_rect, inside_rect) = inside_rect.chop(GUI_LABEL_HEIGHT, GUI_PADDING);
        wr.y = widget_rect.y;
        d.GuiCheckBox(wr, "Left", !keys.bit(5));
        let (widget_rect, inside_rect) = inside_rect.chop(GUI_LABEL_HEIGHT, GUI_PADDING);
        wr.y = widget_rect.y;
        d.GuiCheckBox(wr, "Right", !keys.bit(4));
        let (widget_rect, _) = inside_rect.chop(GUI_LABEL_HEIGHT, GUI_PADDING);
        wr.y = widget_rect.y;
        d.GuiCheckBox(wr, "Shoulder-L", !keys.bit(9));

        let mut inside_rect = rect.shave(GUI_PADDING);
        inside_rect.x += rect.width / 2.0;

        let (widget_rect, inside_rect) = inside_rect.chop(GUI_LABEL_HEIGHT, GUI_PADDING);
        wr.x += rect.width / 2.0;
        wr.y = widget_rect.y;
        d.GuiCheckBox(wr, "A", !keys.bit(0));
        let (widget_rect, inside_rect) = inside_rect.chop(GUI_LABEL_HEIGHT, GUI_PADDING);
        wr.y = widget_rect.y;
        d.GuiCheckBox(wr, "B", !keys.bit(1));
        let (widget_rect, inside_rect) = inside_rect.chop(GUI_LABEL_HEIGHT, GUI_PADDING);
        wr.y = widget_rect.y;
        d.GuiCheckBox(wr, "Start", !keys.bit(3));
        let (widget_rect, inside_rect) = inside_rect.chop(GUI_LABEL_HEIGHT, GUI_PADDING);
        wr.y = widget_rect.y;
        d.GuiCheckBox(wr, "Select", !keys.bit(2));
        let (widget_rect, inside_rect) = inside_rect.chop(GUI_LABEL_HEIGHT, GUI_PADDING);
        wr.y = widget_rect.y;
        d.GuiCheckBox(wr, "Shoulder-R", !keys.bit(8));
        let (_, inside_rect) = inside_rect.chop(GUI_LABEL_HEIGHT, GUI_PADDING);

        let (state_rect, adv_rect) = rect.chop((inside_rect.y - rect.y) as i32, GUI_PADDING);
        d.GuiGroupBox(state_rect, "Keypad State");

        adv_rect
    }

    fn draw_audio_state(
        &self,
        d: &mut Raylib,
        rect: Rectangle,
        gba: &mut Gba<EmulatorState>,
    ) -> Rectangle {
        let inside_rect = rect.shave(GUI_PADDING);

        let (widget_rect, inside_rect) = inside_rect.chop(GUI_LABEL_HEIGHT, GUI_PADDING);

        let fifo_size = gba.sysbus.io.sound.dma_sound[0].fifo.count as f32;

        //   float fifo_size = sb_ring_buffer_size(&emu_state.audio_ring_buff);
        //   GuiLabel(widget_rect, TextFormat("FIFO Size: %4f (%4f)", fifo_size,fifo_size/SB_AUDIO_RING_BUFFER_SIZE));
        d.GuiLabel(
            widget_rect,
            &format!("FIFO Size: {fifo_size} ({})", fifo_size / 32.0),
        );

        inside_rect

        //   sb_vertical_adv(inside_rect, GUI_ROW_HEIGHT, GUI_PADDING, &widget_rect, &inside_rect);
        //   GuiProgressBar(widget_rect, "", "", fifo_size/SB_AUDIO_RING_BUFFER_SIZE, 0, 1);
        //   for(int i=0;i<4;++i){
        //     inside_rect = sb_draw_label(inside_rect,TextFormat("Channel %d",i+1));
        //     sb_vertical_adv(inside_rect, GUI_ROW_HEIGHT, GUI_PADDING, &widget_rect, &inside_rect);
        //     GuiProgressBar(widget_rect, "", "", emu_state.audio_channel_output[i], 0, 1);
        //   }
        //   if(emu_state.system==SYSTEM_GBA){
        //     inside_rect = sb_draw_label(inside_rect,TextFormat("FIFO Channel A"));
        //     sb_vertical_adv(inside_rect, GUI_ROW_HEIGHT, GUI_PADDING, &widget_rect, &inside_rect);
        //     GuiProgressBar(widget_rect, "", "", emu_state.audio_channel_output[4], 0, 1);

        //     inside_rect = sb_draw_label(inside_rect,TextFormat("FIFO Channel B"));
        //     sb_vertical_adv(inside_rect, GUI_ROW_HEIGHT, GUI_PADDING, &widget_rect, &inside_rect);
        //     GuiProgressBar(widget_rect, "", "", emu_state.audio_channel_output[5], 0, 1);
        //   }

        //   inside_rect = sb_draw_label(inside_rect, "Mix Volume (R)");
        //   sb_vertical_adv(inside_rect, GUI_ROW_HEIGHT, GUI_PADDING, &widget_rect, &inside_rect);
        //   GuiProgressBar(widget_rect, "", "", emu_state.mix_r_volume, 0, 1);

        //   inside_rect = sb_draw_label(inside_rect, "Mix Volume (L)");
        //   sb_vertical_adv(inside_rect, GUI_ROW_HEIGHT, GUI_PADDING, &widget_rect, &inside_rect);
        //   GuiProgressBar(widget_rect, "", "", emu_state.mix_l_volume, 0, 1);

        //   inside_rect = sb_draw_label(inside_rect, "Output Waveform");

        //   sb_vertical_adv(inside_rect, 128, GUI_PADDING, &widget_rect, &inside_rect);

        //   Color outline_color = GetColor(GuiGetStyle(DEFAULT,BORDER_COLOR_NORMAL));
        //   Color line_color = GetColor(GuiGetStyle(DEFAULT,BORDER_COLOR_FOCUSED));
        //   DrawRectangleLines(widget_rect.x,widget_rect.y,widget_rect.width,widget_rect.height,outline_color);
        //   int old_v = 0;
        //   static Vector2 points[512];
        //   for(int i=0;i<widget_rect.width;++i){
        //     int entry = (emu_state.audio_ring_buff.read_ptr+i)%SB_AUDIO_RING_BUFFER_SIZE;
        //     int value = emu_state.audio_ring_buff.data[entry]/256/2;
        //     points[i]= (Vector2){widget_rect.x+i,widget_rect.y+64+value};
        //     old_v=value;
        //   }
        //   DrawLineStrip(points,widget_rect.width,line_color);

        //   Rectangle state_rect, adv_rect;
        //   sb_vertical_adv(rect, inside_rect.y - rect.y, GUI_PADDING, &state_rect,
        //                   &adv_rect);

        //   GuiGroupBox(state_rect, "Audio State");
        //   return adv_rect;
    }
}

impl VideoInterface for EmulatorState {
    fn render(&mut self, buffer: &[u8]) {
        println!("UPDATE");
        self.lcd.UpdatePixels(buffer);
    }

    fn poll(&mut self) -> u16 {
        self.keys
    }

    fn push_sample(&mut self, samples: &[i16]) {
        const MAX_SAMPLES: usize = 512;
        const MAX_SAMPLES_PER_UPDATE: usize = 4096;

        // if self.audio.is_audio_stream_processed(&self.audio_stream) {
        //     let mut data = [0; MAX_SAMPLES / std::mem::size_of::<i16>()];
        //     let mut writeBuf = [0i16; MAX_SAMPLES_PER_UPDATE / std::mem::size_of::<i16>()];

        //     let mut waveLength = 1;
        //     let mut readCursor = 0;
        //     let mut writeCursor = 0;

        //     while writeCursor < MAX_SAMPLES_PER_UPDATE / std::mem::size_of::<i16>() {
        //         // Start by trying to write the whole chunk at once
        //         let mut writeLength =
        //             MAX_SAMPLES_PER_UPDATE / std::mem::size_of::<i16>() - writeCursor;

        //         // Limit to the maximum readable size
        //         let readLength = waveLength - readCursor;

        //         if writeLength > readLength {
        //             writeLength = readLength;
        //         }

        //         // Write the slice
        //         &mut writeBuf[writeCursor..writeCursor + writeLength]
        //             .copy_from_slice(&data[readCursor..readCursor + writeLength]);
        //         // memcpy(writeBuf + writeCursor, data + readCursor, writeLength * sizeof(short));

        //         // Update cursors and loop audio
        //         readCursor = (readCursor + writeLength) % waveLength;

        //         writeCursor += writeLength;
        //     }

        //     self.audio_stream.update_audio_stream(samples)
        // }
    }
}
