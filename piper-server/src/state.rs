use std::{
    collections::{HashMap, VecDeque},
    env, io,
    str::FromStr,
    sync::Arc, time::{Duration, Instant},
};

use piper::PiperSynth;
use thiserror::Error;
use tokio::{sync::Mutex, time::sleep};

use crate::models::{ModelError, VoiceModel, get_models};

pub struct VoiceCache {
    max_voices: usize,
    voices: Mutex<VecDeque<CachedVoice>>,
}

struct CachedVoice {
    name: String,
    synth: PiperSynth,
    inserted: Instant
}

#[derive(Clone)]
pub struct AppState {
    pub cache: Arc<VoiceCache>,
    pub default_model: String,
    pub default_speaker: Option<String>,
    pub address: String,
    pub models: HashMap<String, VoiceModel>,
}

impl AppState {
    pub fn load() -> Result<AppState, StateError> {
        _ = dotenvy::dotenv();
        let state = AppState {
            default_model: var("DEFAULT_MODEL")?,
            default_speaker: match var::<String>("DEFAULT_SPEAKER") {
                Ok(s) => Ok(Some(s)),
                Err(e) => match e {
                    ConfigError::MissingVar(_) => Ok(None),
                    _ => Err(e),
                },
            }?,
            address: var_or_default("ADDRESS", || "0.0.0.0:5000".to_string())?,
            models: get_models()?,
            cache: Arc::new(VoiceCache {
                max_voices: 5,
                voices: Default::default(),
            }),
        };

        // confirm default model and speaker are valid
        if let Some(model) = state.models.get(&state.default_model) {
            if let Some(speaker) = &state.default_speaker {
                if !model.speakers.contains_key(speaker) {
                    return Err(StateError::BadDefaultSpeaker(speaker.clone()));
                }
            } else if !model.speakers.is_empty() {
                return Err(StateError::MissingDefaultSpeaker);
            }
        } else {
            return Err(StateError::BadDefaultModel(state.default_model));
        }

        tokio::task::spawn(VoiceCache::cache_invalidate_task(state.cache.clone()));

        Ok(state)
    }
}

#[derive(thiserror::Error, Debug)]
pub enum StateError {
    #[error("Config Error: {0}")]
    ConfigError(#[from] ConfigError),
    #[error("Model Error: {0}")]
    ModelError(#[from] ModelError),

    #[error("DEFAULT_SPEAKER variable is not set but DEFAULT_MODEL has multiple speakers!")]
    MissingDefaultSpeaker,
    #[error("DEFAULT_SPEAKER {0} does not exist")]
    BadDefaultSpeaker(String),
    #[error("DEFAULT_MODEL {0} does not exist")]
    BadDefaultModel(String),
}

impl VoiceCache {
    pub async fn pop_synth(&self, model: &VoiceModel) -> PiperSynth {
        let mut voices = self.voices.lock().await;
        for i in 0..voices.len() {
            if voices[i].name == model.name {
                let voice = voices.remove(i).expect("this is within range");
                return voice.synth
            }
        }

        PiperSynth::init(&model.onnx_file, &model.json_file)
    }

    pub async fn push_synth(&self, model: &VoiceModel, synth: PiperSynth) {
        let mut voices = self.voices.lock().await;
        if voices.len() == self.max_voices {
            voices.pop_front().expect("max_voices should be more than 0");
        }

        voices.push_back(CachedVoice {
            synth,
            name: model.name.clone(),
            inserted: Instant::now()
        });
    }

    async fn cache_invalidate_task(cache: Arc<Self>) {
        loop {
            sleep(Duration::from_secs(60 * 5)).await;
            let mut voices = cache.voices.lock().await;
            let mut i = 0;
            while i < voices.len() {
                if Instant::now().duration_since(voices[i].inserted) >= Duration::from_secs(60) {
                    voices.remove(i);
                } else {
                    i += 1;
                }
            }
        }
    }
}

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("missing environment variable {0}")]
    MissingVar(String),
    #[error("failed to parse environmnet variable {0}")]
    BadVar(String),

    #[error("failed to parse secret file {0}")]
    BadSecret(String),
    #[error("failed to read secret file: {0}")]
    SecretIO(#[from] io::Error),
}

fn var<T: FromStr>(var: &str) -> Result<T, ConfigError> {
    if let Ok(s) = env::var(var) {
        if let Ok(t) = T::from_str(&s) {
            return Ok(t);
        } else {
            return Err(ConfigError::BadSecret(s));
        }
    }

    Err(ConfigError::MissingVar(var.to_string()))
}

//fn var_or_secret<T: FromStr>(var: &str, secret_var: &str) -> Result<T, ConfigError> {
//    if let Ok(s) = env::var(var) {
//        if let Ok(t) = T::from_str(&s) {
//            return Ok(t);
//        } else {
//            return Err(ConfigError::BadVar(var.to_string()));
//        }
//    }
//
//    if let Ok(s) = env::var(secret_var) {
//        match fs::read_to_string(&s) {
//            Ok(file) => {
//                if let Ok(t) = T::from_str(&file) {
//                    return Ok(t);
//                } else {
//                    return Err(ConfigError::BadSecret(s));
//                }
//            }
//            Err(e) => return Err(ConfigError::SecretIO(e)),
//        }
//    }
//
//    Err(ConfigError::MissingVar(format!("{var} or {secret_var}")))
//}

fn var_or_default<T: FromStr>(var: &str, default: fn() -> T) -> Result<T, ConfigError> {
    if let Ok(s) = env::var(var) {
        if let Ok(t) = T::from_str(&s) {
            return Ok(t);
        } else {
            return Err(ConfigError::BadVar(var.to_string()));
        }
    }

    Ok(default())
}
