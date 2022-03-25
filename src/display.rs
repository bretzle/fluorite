use crate::gba;
use glow::{HasContext, PixelUnpackData};
use imgui_glow_renderer::AutoRenderer;
use imgui_sdl2_support::SdlPlatform;
use sdl2::event::Event;
use sdl2::video::{GLContext, GLProfile, Window};
use sdl2::{EventPump, Sdl, VideoSubsystem};
use std::cmp::Ordering;
use std::time::{Duration, Instant};

pub struct Display {
    _video: VideoSubsystem,
    _gl_context: GLContext,
    window: Window,
    screen_tex: u32,

    platform: SdlPlatform,
    renderer: AutoRenderer,
    event_pump: EventPump,

    should_close: bool,
    frames_passed: u32,
    prev_frame_time: Duration,
    prev_fps_update_time: Instant,
}

impl Display {
    pub fn new(sdl: &Sdl, imgui: &mut imgui::Context) -> Display {
        let video = sdl.video().unwrap();

        let gl_attr = video.gl_attr();

        gl_attr.set_context_version(3, 3);
        gl_attr.set_context_profile(GLProfile::Core);

        let width = (gba::WIDTH * gba::SCALE) as u32;
        let height = (gba::HEIGHT * gba::SCALE) as u32 + 19;
        let window = video
            .window("GBA Emulator", width, height)
            .allow_highdpi()
            .opengl()
            .position_centered()
            .resizable()
            .build()
            .unwrap();

        let gl_context = window.gl_create_context().unwrap();
        window.gl_make_current(&gl_context).unwrap();

        window.subsystem().gl_set_swap_interval(1).unwrap();

        let gl = unsafe {
            glow::Context::from_loader_function(|s| window.subsystem().gl_get_proc_address(s) as _)
        };

        let screen_tex = unsafe { gl.create_texture() }.expect("Failed to create GL texture");
        let fbo = unsafe { gl.create_framebuffer() }.expect("Failed to create GL framebuffer");

        unsafe {
            gl.bind_texture(glow::TEXTURE_2D, Some(screen_tex));
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MIN_FILTER,
                glow::LINEAR as _,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MAG_FILTER,
                glow::LINEAR as _,
            );
            gl.tex_storage_2d(
                glow::TEXTURE_2D,
                1,
                glow::RGBA8,
                gba::WIDTH as i32,
                gba::HEIGHT as i32,
            );

            gl.bind_framebuffer(glow::READ_FRAMEBUFFER, Some(fbo));
            gl.framebuffer_texture_2d(
                glow::READ_FRAMEBUFFER,
                glow::COLOR_ATTACHMENT0,
                glow::TEXTURE_2D,
                Some(screen_tex),
                0,
            );
        }

        let platform = SdlPlatform::init(imgui);
        let renderer = AutoRenderer::initialize(gl, imgui).unwrap();
        let event_pump = sdl.event_pump().unwrap();

        Self {
            _video: video,
            _gl_context: gl_context,
            window,
            screen_tex,
            platform,
            renderer,
            event_pump,
            should_close: false,
            frames_passed: 0,
            prev_frame_time: Duration::ZERO,
            prev_fps_update_time: Instant::now(),
        }
    }

    pub fn should_close(&self) -> bool {
        self.should_close
    }

    pub fn render<F>(&mut self, pixels: &[u16], emu_fps: f32, imgui: &mut imgui::Context, draw: F)
    where
        F: FnOnce(&imgui::Ui),
    {
        let begin = Instant::now();

        let (width, height) = {
            let (w, h) = self.window.size();
            (w as i32, h as i32 - 19)
        };

        const HEIGHT: i32 = gba::HEIGHT as i32;
        const WIDTH: i32 = gba::WIDTH as i32;

        let (tex_x, tex_y) = match (width * HEIGHT).cmp(&(height * WIDTH)) {
            Ordering::Greater => {
                let scaled_width = (WIDTH as f32 / HEIGHT as f32 * height as f32) as i32;
                ((width - scaled_width) / 2, 0)
            }
            Ordering::Less => {
                let scaled_height = (HEIGHT as f32 / WIDTH as f32 * width as f32) as i32;
                (0, (height - scaled_height) / 2)
            }
            Ordering::Equal => (0, 0),
        };

        unsafe {
            let gl = self.renderer.gl_context();
            gl.bind_texture(glow::TEXTURE_2D, Some(self.screen_tex));
            gl.clear(glow::COLOR_BUFFER_BIT);
            gl.tex_sub_image_2d(
                glow::TEXTURE_2D,
                0,
                0,
                0,
                gba::WIDTH as i32,
                gba::HEIGHT as i32,
                glow::RGBA,
                glow::UNSIGNED_SHORT_1_5_5_5_REV,
                PixelUnpackData::Slice({
                    let len = pixels.len() * 2;
                    let ptr = pixels.as_ptr() as *const u8;
                    std::slice::from_raw_parts(ptr, len)
                }),
            );
            gl.blit_framebuffer(
                0,
                0,
                gba::WIDTH as i32,
                gba::HEIGHT as i32,
                tex_x,
                height - tex_y,
                width - tex_x,
                tex_y,
                glow::COLOR_BUFFER_BIT,
                glow::NEAREST,
            );
        };

        for event in self.event_pump.poll_iter() {
            self.platform.handle_event(imgui, &event);

            match event {
                Event::Quit { .. } => self.should_close = true,
                Event::KeyDown { .. } => {}
                Event::DropFile { .. } => todo!(),
                _ => {}
            }
        }

        self.platform
            .prepare_frame(imgui, &self.window, &self.event_pump);

        draw(imgui.new_frame());

        self.renderer.render(imgui.render()).unwrap();

        self.window.gl_swap_window();

        self.prev_frame_time = begin.elapsed();
        self.frames_passed += 1;
        let time_passed = self.prev_fps_update_time.elapsed().as_secs_f64();
        if time_passed >= 1.0 {
            let fps = self.frames_passed as f64 / time_passed;
            self.window
                .set_title(&format!("GBA Emulator - {:.2} FPS [{emu_fps:.2}]", fps))
                .expect("Failed to update title");
            self.frames_passed = 0;
            self.prev_fps_update_time = Instant::now();
        }
    }
}

// extern "system" fn gl_debug_callback(
//     _source: u32,
//     _type: u32,
//     _id: u32,
//     sev: u32,
//     _len: i32,
//     message: *const i8,
//     _param: *mut std::ffi::c_void,
// ) {
//     if sev == gl::DEBUG_SEVERITY_NOTIFICATION {
//         return;
//     }

//     unsafe {
//         let message = std::ffi::CStr::from_ptr(message).to_str().unwrap();
//         panic!("OpenGL Debug message: {}", message);
//     }
// }
