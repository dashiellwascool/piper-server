use axum::{
    extract::{Query, State}, http::{header::{self}, StatusCode}, response::IntoResponse, routing::get, Json, Router
};
use piper::{init_piper, PiperOptions};
use serde::Deserialize;
use serde_json::{json, Value};
use tracing::info;

use crate::{errors::make_error, state::AppState};

mod errors;
mod models;
mod state;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    info!("Piper Server v{}", env!("CARGO_PKG_VERSION"));
    info!("Made with ❤ by Dashiell");

    info!("Initializing Piper");
    init_piper();

    let state = AppState::load()?;
    let addr = state.address.clone();

    let app = Router::new()
        .route("/models", get(get_models))
        .route("/speakers", get(get_speakers))
        .route("/speak", get(speak))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!("Listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await?;

    Ok(())
}

async fn get_models(State(state): State<AppState>) -> Json<Vec<String>> {
    Json::from(
        state
            .models
            .keys()
            .map(|s| s.to_owned())
            .collect::<Vec<String>>(),
    )
}

#[derive(Deserialize)]
struct SpeakersQuery {
    model: Option<String>,
}

async fn get_speakers(
    State(state): State<AppState>,
    Query(model): Query<SpeakersQuery>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let model = model.model.as_ref().unwrap_or(&state.default_model);

    let model = state
        .models
        .get(model)
        .ok_or_else(|| make_error(StatusCode::BAD_REQUEST, "Invalid model"))?;

    Ok(json!({
        "model": model.name,
        "speakers": model.speakers.keys().map(|s| s.to_owned()).collect::<Vec<String>>()
    }).into())
}

#[derive(Deserialize)]
struct SpeakBody {
    text: String,
    model: Option<String>,
    speaker: Option<String>,
}

async fn speak(
    State(state): State<AppState>,
    json: Json<SpeakBody>,
) -> Result<impl IntoResponse, (StatusCode, Json<Value>)> {
    let model_data = state
        .models
        .get(json.model.as_ref().unwrap_or(&state.default_model))
        .ok_or_else(|| make_error(StatusCode::BAD_REQUEST, "Invalid model"))?;

    // get speaker id
    let speaker_id = if let Some(speaker) = &json.speaker {
        *model_data.speakers.get(speaker).unwrap_or(&0)
    } else {
        0
    };

    let options = PiperOptions {
        length_scale: Some(1.0),
        noise_scale: Some(model_data.suggested_settings.noise_scale),
        noise_w_scale: Some(model_data.suggested_settings.noise_w),
        speaker_id: Some(speaker_id),
    };

    // process the actual text
    let synth = state.cache.pop_synth(model_data).await;
    let data = synth.synthesize_to_wav(&json.text, Some(options)).map_err(errors::anyhow_internal_error)?;
    let header = [
        (header::CONTENT_TYPE, "audio/wav".to_owned()),
    ];
    state.cache.push_synth(model_data, synth).await;
    Ok((header, data))
}
