use axum::{http::StatusCode, Json};
use serde_json::{json, Value};
use tracing::{error, warn};

pub const INTERNAL_ERROR_CODE: i32 = -1;
pub const BAD_MODEL: i32 = 1;

pub fn make_error(code: StatusCode, error_code: i32, message: &str) -> (StatusCode, Json<Value>) {
    if code.is_server_error() {
        error!("{code}: {message}");
    } else {
        warn!("{code}: {message}");
    }
    let response = json!({
        "error": code.as_u16(),
        "error_code": error_code,
        "message": message.to_string()
    });

    (code, Json(response))
}

//pub fn internal_error<E: std::error::Error>(err: E) -> (StatusCode, Json<Value>) {
//    make_error(StatusCode::INTERNAL_SERVER_ERROR, &err.to_string())
//}

pub fn anyhow_internal_error(err: anyhow::Error) -> (StatusCode, Json<Value>) {
    make_error(StatusCode::INTERNAL_SERVER_ERROR, INTERNAL_ERROR_CODE, &err.to_string())
}
