use axum::{http::StatusCode, Json};
use serde_json::{json, Value};
use tracing::{error, warn};

pub fn make_error(code: StatusCode, message: &str) -> (StatusCode, Json<Value>) {
    if code.is_server_error() {
        error!("{code}: {message}");
    } else {
        warn!("{code}: {message}");
    }
    let response = json!({
        "error": code.as_u16(),
        "message": message.to_string()
    });

    (code, Json(response))
}

//pub fn internal_error<E: std::error::Error>(err: E) -> (StatusCode, Json<Value>) {
//    make_error(StatusCode::INTERNAL_SERVER_ERROR, &err.to_string())
//}

pub fn anyhow_internal_error(err: anyhow::Error) -> (StatusCode, Json<Value>) {
    make_error(StatusCode::INTERNAL_SERVER_ERROR, &err.to_string())
}
