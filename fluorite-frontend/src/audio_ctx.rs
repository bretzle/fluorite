use std::ffi::c_void;
use std::mem::MaybeUninit;
use std::ptr;
use std::sync::Mutex;

use fluorite_gba::AudioInterface;
use sdl2::{
    get_error,
    sys::{
        SDL_AudioSpec, SDL_AudioStream, SDL_AudioStreamAvailable, SDL_AudioStreamClear,
        SDL_AudioStreamGet, SDL_AudioStreamPut, SDL_NewAudioStream, SDL_OpenAudioDevice,
        SDL_PauseAudioDevice, AUDIO_F32SYS, AUDIO_S16,
    },
    AudioSubsystem, Sdl,
};

const WANT_FREQUENCY: u32 = 44100;
const WANT_CHANNELS: u32 = 2;
const SECOND: u32 = WANT_FREQUENCY * WANT_CHANNELS * 4;
const FLOAT_SIZE: i32 = std::mem::size_of::<f32>() as i32;

pub struct AudioCtx {
    _audio: AudioSubsystem,
    device: u32,
    stream: *mut SDL_AudioStream,
    lock: Mutex<()>,
}

impl AudioCtx {
    pub fn new(sdl: &Sdl) -> Self {
        Self {
            _audio: sdl.audio().unwrap(),
            device: 0,
            stream: ptr::null_mut(),
            lock: Mutex::new(()),
        }
    }

    pub fn init(&mut self) {
        unsafe {
            let want = SDL_AudioSpec {
                freq: WANT_FREQUENCY as i32,
                format: AUDIO_F32SYS as u16,
                channels: WANT_CHANNELS as u8,
                silence: 0,
                samples: 1024,
                padding: 0,
                size: 0,
                callback: Some(Self::callback),
                userdata: self as *mut _ as *mut c_void,
            };
            let mut have = MaybeUninit::uninit();
            match SDL_OpenAudioDevice(ptr::null(), 0, &want, have.as_mut_ptr(), 0) {
                0 => panic!("Cannot init audio device: {}", get_error()),
                id => self.device = id,
            }

            let have = have.assume_init();

            self.stream = SDL_NewAudioStream(
                AUDIO_S16 as u16,
                2,
                32 * 1024,
                have.format,
                have.channels,
                have.freq,
            );

            if self.stream.is_null() {
                panic!("Cannot init audio stream: {}", get_error())
            }
        }
    }

    pub fn pause(&mut self) {
        let _guard = self.lock.lock().unwrap();
        unsafe {
            SDL_AudioStreamClear(self.stream);
            SDL_PauseAudioDevice(self.device, 1);
        }
    }

    pub fn resume(&mut self) {
        unsafe {
            SDL_PauseAudioDevice(self.device, 0);
        }
    }

    fn write_impl(&mut self, mut samples: [i16; 2]) {
        let _guard = self.lock.lock().unwrap();
        unsafe {
            if SDL_AudioStreamAvailable(self.stream) < (SECOND / 8) as i32 {
                // TODO: link this with config.volume and config.mute
                samples[0] /= 2;
                samples[1] /= 2;
                SDL_AudioStreamPut(self.stream, samples.as_ptr() as *const _, 4);
            }
        }
    }

    unsafe extern "C" fn callback(data: *mut c_void, stream: *mut u8, length: i32) {
        let ctx = &mut *(data as *mut AudioCtx);

        let mut gotten = 0;
        {
            let _guard = ctx.lock.lock().unwrap();
            if SDL_AudioStreamAvailable(ctx.stream) != 0 {
                gotten = SDL_AudioStreamGet(ctx.stream, stream as *mut _, length);
            }
        }

        if gotten == -1 {
            // memset(stream, 0, length);
            for idx in 0..length as usize {
                *stream.add(idx) = 0;
            }
            return;
        }

        if gotten < length {
            let mut f_sample = 0.0;
            let f_stream = stream as *mut f32;
            let f_gotten = (gotten / FLOAT_SIZE) as usize;
            let f_length = (length / FLOAT_SIZE) as usize;

            if f_gotten != 0 {
                f_sample = *f_stream.add(f_gotten - 1);
            }

            for i in f_gotten..f_length {
                *f_stream.add(i) = f_sample;
            }
        }
    }
}

impl AudioInterface for AudioCtx {
    fn write(&mut self, samples: [i16; 2]) {
        self.write_impl(samples)
    }
}
