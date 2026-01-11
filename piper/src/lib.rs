use std::{
    ffi::CString,
    fs,
    io::Cursor,
    mem::MaybeUninit,
    path::PathBuf,
};

use hound::{SampleFormat, WavSpec};
use rust_embed::Embed;
use thiserror::Error;
use tracing::{error, info};

use crate::libpiper::{
    piper_create, piper_default_synthesize_options, piper_free, piper_synthesize_next, piper_synthesize_start, piper_synthesizer
};

mod libpiper;

const PIPER_OK: i32 = libpiper::PIPER_OK as i32;
const PIPER_DONE: i32 = libpiper::PIPER_DONE as i32;

pub struct PiperSynth(*mut piper_synthesizer);

pub struct PiperOptions {
    pub length_scale: Option<f32>,
    pub noise_scale: Option<f32>,
    pub noise_w_scale: Option<f32>,
    pub speaker_id: Option<i32>,
}

#[derive(Error, Debug)]
pub enum SynthesizeError {
    #[error("synthesise error: {0}")]
    SynthError(i32),
}

#[derive(Embed)]
#[folder = "$OUT_DIR/espeak-ng-data"]
struct EspeakData;

impl Drop for PiperSynth {
    fn drop(&mut self) {
        info!("freeing synth");
        unsafe {
            piper_free(self.0);
        }
    }
}

unsafe impl Send for PiperSynth {}

pub fn init_piper() {
    // copy espeak data out of the binary
    let path = PathBuf::from("libs").join("espeak_data");
    for name in EspeakData::iter() {
        let full_name = path.join(&*name);
        fs::create_dir_all(full_name.parent().unwrap()).unwrap();
        fs::write(full_name, EspeakData::get(&name).unwrap().data).unwrap();
    }
}

impl PiperSynth {
    pub fn init(model_path: &str, config_path: &str) -> PiperSynth {
        info!("loading synth {}", model_path);
        let synth = unsafe {
            let model_path = CString::new(model_path).unwrap();
            let config_path = CString::new(config_path).unwrap();
            let data_path = CString::new("libs/espeak_data").unwrap();

            piper_create(
                model_path.as_ptr(),
                config_path.as_ptr(),
                data_path.as_ptr(),
            )
        };

        PiperSynth(synth)
    }

    pub fn synthesize_to_wav(
        &self,
        message: &str,
        options: Option<PiperOptions>,
    ) -> anyhow::Result<Vec<u8>> {
    let mut opt = unsafe { piper_default_synthesize_options(self.0) };
    if let Some(options) = options {
        opt.length_scale = options.length_scale.unwrap_or(opt.length_scale);
        opt.noise_scale = options.noise_scale.unwrap_or(opt.noise_scale);
        opt.noise_w_scale = options.noise_w_scale.unwrap_or(opt.noise_w_scale);
        opt.speaker_id = options.speaker_id.unwrap_or(opt.speaker_id);
    }

    let message = CString::new(message)?;
    unsafe {
        piper_synthesize_start(self.0, message.as_ptr(), &opt);
    }

    let mut buf = Vec::new();

    let mut chunk = MaybeUninit::uninit();

    // read first buffer so we can setup the header
    let n = unsafe { piper_synthesize_next(self.0, chunk.as_mut_ptr()) };
    if n != PIPER_OK {
        error!("Unexpected return code on first synthesize: {n}");
        return Err(SynthesizeError::SynthError(n).into());
    }

    let mut chunk = unsafe { chunk.assume_init() };

    let spec = WavSpec {
        channels: 1, // is this right?
        sample_rate: chunk.sample_rate as u32,
        bits_per_sample: 32,
        sample_format: SampleFormat::Float,
    };

    let mut writer = hound::WavWriter::new(Cursor::new(&mut buf), spec)?;
    for i in 0..chunk.num_samples {
        writer.write_sample(unsafe { *chunk.samples.add(i) })?;
    }

    // read the rest of them
    loop {
        match unsafe { piper_synthesize_next(self.0, &mut chunk) } {
            PIPER_OK => {
                for i in 0..chunk.num_samples {
                    writer.write_sample(unsafe { *chunk.samples.add(i) })?;
                }
                writer.flush()?;
            }
            PIPER_DONE => {
                break;
            }
            n => {
                error!("Unexpected return code when synthesizing: {n}");
                return Err(SynthesizeError::SynthError(n).into());
            }
        }
    }

    writer.finalize()?;

    Ok(buf)
    }
}

