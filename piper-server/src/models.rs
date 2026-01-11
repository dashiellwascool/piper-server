use std::{collections::HashMap, fs, io::BufReader, path::PathBuf};

use serde::{Deserialize, Serialize};
use tracing::{info, warn};

#[derive(Clone)]
pub struct VoiceModel {
    pub name: String,
    pub speakers: HashMap<String, i32>,
    pub onnx_file: String,
    pub json_file: String,
    pub suggested_settings: Inference,
}

#[derive(Serialize, Deserialize)]
struct ModelJson {
    pub dataset: String,
    pub inference: Inference,
    pub speaker_id_map: HashMap<String, i32>,
    pub language: Language,
}

#[derive(Serialize, Deserialize)]
struct Language {
    pub code: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Inference {
    pub noise_scale: f32,
    pub length_scale: f32,
    pub noise_w: f32,
}

#[derive(thiserror::Error, Debug)]
pub enum ModelError {
    #[error("IO Error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Json Error: {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("Path is not unicode {0}")]
    BadPath(PathBuf),
}

pub fn get_models() -> Result<HashMap<String, VoiceModel>, ModelError> {
    let mut models = HashMap::new();

    for entry in fs::read_dir("voices")? {
        let entry = entry?;
        if !entry.path().is_file() {
            warn!(
                "{} is not a file. Directories in voices/ are not searched!",
                entry.path().to_string_lossy()
            );
            continue;
        }
        if let Some(extension) = entry.path().extension()
            && extension.eq_ignore_ascii_case("onnx")
        {
            // first let's confirm there's a matching onnx.json file
            let json_path = entry.path().with_extension("onnx.json");
            if !fs::exists(&json_path).unwrap_or(false) {
                warn!(
                    "found onnx model {} without a matching onnx.json file",
                    entry.path().to_string_lossy()
                );
                continue;
            }

            // read json file
            let file = fs::File::open(&json_path)?;
            let model_json: ModelJson = serde_json::from_reader(BufReader::new(file))?;
            let model = VoiceModel {
                name: format!("{} {}", model_json.language.code, model_json.dataset),
                speakers: model_json.speaker_id_map,
                onnx_file: entry
                    .path()
                    .to_str()
                    .ok_or_else(|| ModelError::BadPath(entry.path()))?
                    .to_owned(),
                json_file: json_path
                    .to_str()
                    .ok_or_else(|| ModelError::BadPath(json_path.clone()))?
                    .to_owned(),
                suggested_settings: model_json.inference.clone(),
            };
            info!("Found model {}", model.name);
            if let Some(m) = models.insert(model.name.clone(), model) {
                warn!("There are multiple voice models with name {}!", m.name);
            }
        }
    }

    Ok(models)
}
