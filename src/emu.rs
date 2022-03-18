use fluorite_arm::registers::CpuState;
use fluorite_common::BitIndex;
use fluorite_gba::gba::Gba;
use fluorite_gba::keypad::{Keys, KEYINPUT_ALL_RELEASED};
use fluorite_gba::sysbus::Bus;
use fluorite_gba::VideoInterface;
use raylib::prelude::*;

use crate::consts::*;
use crate::utils::RectExt;

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
        const SCREEN_WIDTH: f32 = 1390.0; // rl.GetScreenWidth() as f32;
        const SCREEN_HEIGHT: f32 = 640.0; // rl.GetScreenHeight() as f32;

        let keys = gba.sysbus.read_16(0x04000130);

        rl.draw(move |d| {
            d.clear_background(Color::WHITE);

            const PANEL_WIDTH: f32 = 430.0;

            // draw side panel
            let rect = Rectangle::new(0.0, 0.0, PANEL_WIDTH, SCREEN_HEIGHT);
            let rect_inside = rect.shave(GUI_PADDING);

            // draw emu state
            let mut rect_inside = {
                let (mut widget_rect, inside_rect) = rect_inside
                    .shave(GUI_PADDING)
                    .chop(GUI_ROW_HEIGHT, GUI_PADDING);

                widget_rect.width =
                    widget_rect.width / 4.0 - d.get_style().toggle.text_spacing as f32;

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

                let (state_rect, adv_rect) =
                    rect_inside.chop((inside_rect.y - rect_inside.y) as i32, GUI_PADDING);
                // TODO: get avg frame time from emulator state
                d.GuiGroupBox(state_rect, &format!("Emulator State [FPS: {}]", self.fps));

                adv_rect
            };

            self.last_rect.x = 0.0;
            self.last_rect.y = 0.0;
            self.last_rect.width = if self.last_rect.height < rect_inside.height {
                rect_inside.width - 5.0
            } else {
                rect_inside.width - d.get_style().listview.scrollbar_width as f32 - 5.0
            };

            let (view, scroll) = d.GuiScrollPanel(rect_inside, self.last_rect, self.scroll);
            self.scroll = scroll;

            rect_inside.y += self.scroll.y;
            rect_inside.y += GUI_PADDING as f32;
            rect_inside.x += GUI_PADDING as f32;
            rect_inside.width = view.width - GUI_PADDING as f32 * 1.5;

            d.scissor(
                view.x as i32,
                view.y as i32,
                view.width as i32,
                view.height as i32,
                |d| {
                    LayoutBuilder::new(&mut rect_inside)
                        .block(GUI_LABEL_HEIGHT, GUI_PADDING, |mut block: Block| {
                            let [left, right] = block.columns::<2>();

                            left.padding(GUI_PADDING as f32)
                                .widget(|wr| {
                                    d.GuiCheckBox(wr, "Up", !keys.bit(6));
                                })
                                .widget(|wr| {
                                    d.GuiCheckBox(wr, "Down", !keys.bit(7));
                                })
                                .widget(|wr| {
                                    d.GuiCheckBox(wr, "Left", !keys.bit(5));
                                })
                                .widget(|wr| {
                                    d.GuiCheckBox(wr, "Right", !keys.bit(4));
                                })
                                .widget(|wr| {
                                    d.GuiCheckBox(wr, "Shoulder-L", !keys.bit(9));
                                })
                                .finish();

                            block.content = right
                                .padding(GUI_PADDING as f32)
                                .widget(|wr| {
                                    d.GuiCheckBox(wr, "A", !keys.bit(0));
                                })
                                .widget(|wr| {
                                    d.GuiCheckBox(wr, "B", !keys.bit(1));
                                })
                                .widget(|wr| {
                                    d.GuiCheckBox(wr, "Start", !keys.bit(3));
                                })
                                .widget(|wr| {
                                    d.GuiCheckBox(wr, "Select", !keys.bit(2));
                                })
                                .widget(|wr| {
                                    d.GuiCheckBox(wr, "Shoulder-R", !keys.bit(8));
                                })
                                .finish();

                            block
                                .cover(|rect| {
                                    d.GuiGroupBox(rect, "Keypad State");
                                })
                                .finish()
                        })
                        .block(GUI_LABEL_HEIGHT, GUI_PADDING, |block: Block| {
                            self.last_rect = block.orig;
                            self.last_rect.height += 184.0;
                            let arm = gba.arm_cpu();
                            const REG_NAMES: [&str; 21] = [
                                "R0", "R1", "R2", "R3", "R4", "R5", "R6", "R7", "R8", "R9", "R10",
                                "R11", "R12", "R13", "R14", "R15(PC)", "CPSR", "N", "Z", "C", "V",
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

                            let [mut left, mut right] =
                                block.padding(GUI_PADDING as f32).columns::<2>();

                            left.panel(|rect| {
                                let values = reg_vals;
                                let mut inside_rect = rect.shave(GUI_PADDING);

                                left.content.height = 5.0;

                                for i in 0..21 {
                                    let (mut widget_rect, new_inside_rect) =
                                        inside_rect.chop(GUI_LABEL_HEIGHT, GUI_PADDING + 5);

                                    d.GuiLabel(widget_rect, REG_NAMES[i]);
                                    let w =
                                        (new_inside_rect.width - GUI_PADDING as f32 * 2.0) / 3.0;
                                    widget_rect.x += w;
                                    d.GuiLabel(widget_rect, &format!("0x{:X}", values[i]));

                                    widget_rect.x += w + GUI_PADDING as f32 * 2.0;
                                    d.GuiLabel(widget_rect, &format!("{}", values[i]));

                                    left.content.height +=
                                        (GUI_LABEL_HEIGHT + GUI_PADDING + 5) as f32;

                                    inside_rect = new_inside_rect;
                                }

                                d.GuiGroupBox(left.content, "Registers");
                            })
                            .finish();

                            right
                                .panel(|mut rect| {
                                    rect.width -= 5.0;
                                    rect.x += 5.0;
                                    let values = reg_vals;
                                    let mut inside_rect = rect.shave(GUI_PADDING);

                                    right.content.height = 5.0;

                                    for i in 0..21 {
                                        let (mut widget_rect, new_inside_rect) =
                                            inside_rect.chop(GUI_LABEL_HEIGHT, GUI_PADDING + 5);

                                        d.GuiLabel(widget_rect, REG_NAMES[i]);
                                        let w = (new_inside_rect.width - GUI_PADDING as f32 * 2.0)
                                            / 3.0;
                                        widget_rect.x += w;
                                        d.GuiLabel(widget_rect, &format!("0x{:X}", values[i]));

                                        widget_rect.x += w + GUI_PADDING as f32 * 2.0;
                                        d.GuiLabel(widget_rect, &format!("{}", values[i]));

                                        right.content.height +=
                                            (GUI_LABEL_HEIGHT + GUI_PADDING + 5) as f32;

                                        inside_rect = new_inside_rect;
                                    }

                                    d.GuiGroupBox(right.content, "Banked Registers");
                                })
                                .finish();

                            block
                                .modify(|block| {
                                    let (_, bot) =
                                        block.orig.chop(right.content.height as i32, GUI_PADDING);
                                    block.content = bot;
                                })
                                .padding(GUI_PADDING as f32)
                                .panel(|rect| {
                                    let mut inside_rect = rect.shave(GUI_PADDING);
                                    let arm = gba.arm_cpu_mut();
                                    let pc = arm.get_reg(15);

                                    // TODO: disassemble THUMB

                                    let state = arm.get_cpu_state();
                                    let mut disasm = String::with_capacity(64);

                                    for i in -6..5i32 {
                                        let (mut widget_rect, new_inside_rect) =
                                            inside_rect.chop(GUI_LABEL_HEIGHT, GUI_PADDING + 5);

                                        let pc_render = i
                                            * (if state == CpuState::THUMB { 2 } else { 4 })
                                            + pc as i32
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

                                            let opcode = arm
                                                .get_instructionge(pc_render as u32, &mut disasm);
                                            d.GuiLabel(widget_rect, disasm.as_str());
                                            disasm.clear();

                                            widget_rect.x += 150.0;
                                            d.GuiLabel(
                                                widget_rect,
                                                &match state {
                                                    CpuState::ARM => format!("{:08X}", opcode),
                                                    CpuState::THUMB => {
                                                        format!("{:04X}", opcode)
                                                    }
                                                },
                                            );
                                            widget_rect.x += 50.0;
                                        }

                                        inside_rect = new_inside_rect
                                    }

                                    let (state_rect, _) =
                                        rect.chop((inside_rect.y - rect.y) as i32, GUI_PADDING);
                                    d.GuiGroupBox(state_rect, &format!("Instructions [{}]", state));
                                })
                                .cover(|_| {
                                    let mut rect = block.content;
                                    rect.height += 85.0;
                                    d.GuiGroupBox(rect, "ARM7 State");
                                })
                                .finish()
                        })
                        .build();
                },
            );

            // draw lcd screen
            const LCD_RECT: Rectangle =
                Rectangle::new(PANEL_WIDTH, 0.0, SCREEN_WIDTH - PANEL_WIDTH, SCREEN_HEIGHT);
            let tex = &self.lcd;
            tex.SetTextureFilter(TextureFilter::Point);
            d.DrawTextureQuad(tex, Vector2::ONE, Vector2::ZERO, LCD_RECT, Color::WHITE);
        });
    }

    // fn draw_io_state(
    //     &self,
    //     d: &mut Raylib,
    //     rect: Rectangle,
    //     gba: &mut Gba<EmulatorState>,
    // ) -> Rectangle {
    //     let mut rect = rect;

    //     for reg in IO_REGS {
    //         let mut r = rect.shave(GUI_PADDING);
    //         let addr = reg.addr;
    //         let data = gba.sysbus.read_16(addr);
    //         // let mut has_fields = false;
    //         for bit in reg.bits {
    //             if let Some(bit) = bit {
    //                 let start = bit.start;
    //                 let size = bit.size;
    //                 if size > 0 {
    //                     let field_data = bfe!(data, start, size);
    //                     // has_fields = true;
    //                     let mut r2 = r;
    //                     if size > 1 {
    //                         r = d.draw_label(r, &format!("[{}:{}]:", start, start + size - 1));
    //                     } else {
    //                         r = d.draw_label(r, &format!("{}:", start));
    //                     }

    //                     r2.x += 30.0;
    //                     d.draw_label(r2, &format!("{}", field_data));
    //                     r2.x += 25.0;
    //                     d.draw_label(r2, bit.name);
    //                 }
    //             }
    //         }

    //         let (state_rect, adv_rect) = rect.chop(r.y as i32 - rect.y as i32, GUI_PADDING);
    //         d.GuiGroupBox(
    //             state_rect,
    //             &format!("{}({:X}): {:04X}", reg.name, addr, data),
    //         );
    //         rect = adv_rect;
    //     }

    //     rect
    // }

    // fn draw_audio_state(
    //     &self,
    //     d: &mut Raylib,
    //     rect: Rectangle,
    //     gba: &mut Gba<EmulatorState>,
    // ) -> Rectangle {
    //     let inside_rect = rect.shave(GUI_PADDING);

    //     let (widget_rect, inside_rect) = inside_rect.chop(GUI_LABEL_HEIGHT, GUI_PADDING);

    //     let fifo_size = gba.sysbus.io.sound.dma_sound[0].fifo.count as f32;

    //     //   float fifo_size = sb_ring_buffer_size(&emu_state.audio_ring_buff);
    //     //   GuiLabel(widget_rect, TextFormat("FIFO Size: %4f (%4f)", fifo_size,fifo_size/SB_AUDIO_RING_BUFFER_SIZE));
    //     d.GuiLabel(
    //         widget_rect,
    //         &format!("FIFO Size: {fifo_size} ({})", fifo_size / 32.0),
    //     );

    //     inside_rect

    //     //   sb_vertical_adv(inside_rect, GUI_ROW_HEIGHT, GUI_PADDING, &widget_rect, &inside_rect);
    //     //   GuiProgressBar(widget_rect, "", "", fifo_size/SB_AUDIO_RING_BUFFER_SIZE, 0, 1);
    //     //   for(int i=0;i<4;++i){
    //     //     inside_rect = sb_draw_label(inside_rect,TextFormat("Channel %d",i+1));
    //     //     sb_vertical_adv(inside_rect, GUI_ROW_HEIGHT, GUI_PADDING, &widget_rect, &inside_rect);
    //     //     GuiProgressBar(widget_rect, "", "", emu_state.audio_channel_output[i], 0, 1);
    //     //   }
    //     //   if(emu_state.system==SYSTEM_GBA){
    //     //     inside_rect = sb_draw_label(inside_rect,TextFormat("FIFO Channel A"));
    //     //     sb_vertical_adv(inside_rect, GUI_ROW_HEIGHT, GUI_PADDING, &widget_rect, &inside_rect);
    //     //     GuiProgressBar(widget_rect, "", "", emu_state.audio_channel_output[4], 0, 1);

    //     //     inside_rect = sb_draw_label(inside_rect,TextFormat("FIFO Channel B"));
    //     //     sb_vertical_adv(inside_rect, GUI_ROW_HEIGHT, GUI_PADDING, &widget_rect, &inside_rect);
    //     //     GuiProgressBar(widget_rect, "", "", emu_state.audio_channel_output[5], 0, 1);
    //     //   }

    //     //   inside_rect = sb_draw_label(inside_rect, "Mix Volume (R)");
    //     //   sb_vertical_adv(inside_rect, GUI_ROW_HEIGHT, GUI_PADDING, &widget_rect, &inside_rect);
    //     //   GuiProgressBar(widget_rect, "", "", emu_state.mix_r_volume, 0, 1);

    //     //   inside_rect = sb_draw_label(inside_rect, "Mix Volume (L)");
    //     //   sb_vertical_adv(inside_rect, GUI_ROW_HEIGHT, GUI_PADDING, &widget_rect, &inside_rect);
    //     //   GuiProgressBar(widget_rect, "", "", emu_state.mix_l_volume, 0, 1);

    //     //   inside_rect = sb_draw_label(inside_rect, "Output Waveform");

    //     //   sb_vertical_adv(inside_rect, 128, GUI_PADDING, &widget_rect, &inside_rect);

    //     //   Color outline_color = GetColor(GuiGetStyle(DEFAULT,BORDER_COLOR_NORMAL));
    //     //   Color line_color = GetColor(GuiGetStyle(DEFAULT,BORDER_COLOR_FOCUSED));
    //     //   DrawRectangleLines(widget_rect.x,widget_rect.y,widget_rect.width,widget_rect.height,outline_color);
    //     //   int old_v = 0;
    //     //   static Vector2 points[512];
    //     //   for(int i=0;i<widget_rect.width;++i){
    //     //     int entry = (emu_state.audio_ring_buff.read_ptr+i)%SB_AUDIO_RING_BUFFER_SIZE;
    //     //     int value = emu_state.audio_ring_buff.data[entry]/256/2;
    //     //     points[i]= (Vector2){widget_rect.x+i,widget_rect.y+64+value};
    //     //     old_v=value;
    //     //   }
    //     //   DrawLineStrip(points,widget_rect.width,line_color);

    //     //   Rectangle state_rect, adv_rect;
    //     //   sb_vertical_adv(rect, inside_rect.y - rect.y, GUI_PADDING, &state_rect,
    //     //                   &adv_rect);

    //     //   GuiGroupBox(state_rect, "Audio State");
    //     //   return adv_rect;
    // }
}

impl VideoInterface for EmulatorState {
    fn render(&mut self, buffer: &[u8]) {
        println!("UPDATE");
        self.lcd.UpdatePixels(buffer);
    }

    fn poll(&mut self) -> u16 {
        self.keys
    }

    fn push_sample(&mut self, _samples: &[i16]) {
        // const MAX_SAMPLES: usize = 512;
        // const MAX_SAMPLES_PER_UPDATE: usize = 4096;

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
