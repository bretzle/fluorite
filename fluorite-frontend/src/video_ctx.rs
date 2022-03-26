use fluorite_gba::consts::{HEIGHT, SCALE, WIDTH};
use glow::{HasContext, PixelUnpackData};
use imgui::Context;
use imgui_glow_renderer::AutoRenderer;
use imgui_sdl2_support::SdlPlatform;
use sdl2::{
    event::Event,
    video::{GLContext, GLProfile, Window},
    EventPump, Sdl, VideoSubsystem,
};
use std::cmp::Ordering;

pub struct VideoCtx {
    _subsystem: VideoSubsystem,
    _gl_context: GLContext,
    pub window: Window,
    imgui: Context,
    platform: SdlPlatform,
    renderer: AutoRenderer,
    frame_texture: u32,
}

impl VideoCtx {
    pub fn init(sdl: &Sdl) -> Self {
        let video = sdl.video().unwrap();

        let gl_attr = video.gl_attr();

        gl_attr.set_context_version(3, 3);
        gl_attr.set_context_profile(GLProfile::Core);

        let width = (WIDTH * SCALE) as u32;
        let height = (HEIGHT * SCALE) as u32 + 19;
        let mut window = video
            .window("GBA Emulator", width, height)
            .allow_highdpi()
            .opengl()
            .position_centered()
            .resizable()
            .build()
            .unwrap();

        window
            .set_minimum_size(WIDTH as u32, HEIGHT as u32)
            .unwrap();

        let gl_context = window.gl_create_context().unwrap();
        window.gl_make_current(&gl_context).unwrap();

        window.subsystem().gl_set_swap_interval(1).unwrap();

        let gl = unsafe {
            glow::Context::from_loader_function(|s| window.subsystem().gl_get_proc_address(s) as _)
        };

        let frame_texture = unsafe { gl.create_texture() }.expect("Failed to create GL texture");
        let fbo = unsafe { gl.create_framebuffer() }.expect("Failed to create GL framebuffer");

        unsafe {
            gl.bind_texture(glow::TEXTURE_2D, Some(frame_texture));
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
                WIDTH as i32,
                HEIGHT as i32,
            );

            gl.bind_framebuffer(glow::READ_FRAMEBUFFER, Some(fbo));
            gl.framebuffer_texture_2d(
                glow::READ_FRAMEBUFFER,
                glow::COLOR_ATTACHMENT0,
                glow::TEXTURE_2D,
                Some(frame_texture),
                0,
            );
        }

        let mut imgui = Context::create();
        let platform = SdlPlatform::init(&mut imgui);
        let renderer = AutoRenderer::initialize(gl, &mut imgui).unwrap();

        Self {
            _subsystem: video,
            _gl_context: gl_context,
            window,
            imgui,
            platform,
            renderer,
            frame_texture,
        }
    }

    fn render_texture(&self, x: i32, y: i32, width: i32, height: i32, pixels: PixelUnpackData) {
        unsafe {
            let gl = self.renderer.gl_context();
            gl.bind_texture(glow::TEXTURE_2D, Some(self.frame_texture));
            gl.clear(glow::COLOR_BUFFER_BIT);
            gl.tex_sub_image_2d(
                glow::TEXTURE_2D,
                0,
                0,
                0,
                WIDTH as i32,
                HEIGHT as i32,
                glow::RGBA,
                glow::UNSIGNED_SHORT_1_5_5_5_REV,
                pixels,
            );
            gl.blit_framebuffer(
                0,
                0,
                WIDTH as i32,
                HEIGHT as i32,
                x,
                height - y,
                width - x,
                y,
                glow::COLOR_BUFFER_BIT,
                glow::NEAREST,
            );
        };
    }

    pub fn render(&mut self, pixels: &[u16], draw_imgui: bool) {
        let (width, height) = {
            let (w, h) = self.window.size();
            (w as i32, h as i32)
        };

        let (tex_x, tex_y) = match (width * HEIGHT as i32).cmp(&(height * WIDTH as i32)) {
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

        self.render_texture(
            tex_x,
            tex_y,
            width,
            height,
            PixelUnpackData::Slice({
                unsafe {
                    let len = pixels.len() * 2;
                    let ptr = pixels.as_ptr() as *const u8;
                    std::slice::from_raw_parts(ptr, len)
                }
            }),
        );

        if draw_imgui {
            self.renderer.render(self.imgui.render()).unwrap();
        }

        self.window.gl_swap_window();
    }

    pub fn handle_event(&mut self, event: &Event) {
        self.platform.handle_event(&mut self.imgui, event);
    }

    pub fn draw<F: FnOnce(&imgui::Ui)>(&mut self, event_pump: &EventPump, draw_fn: F) {
        self.platform
            .prepare_frame(&mut self.imgui, &self.window, event_pump);

        let ui = self.imgui.new_frame();
        draw_fn(ui);
    }
}
