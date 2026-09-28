use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;
use std::fmt;

#[derive(Debug)]
pub enum AppError {
    Io(std::io::Error),
    Json(serde_json::Error),
    Lean(String),
    Protocol(String),
    Internal(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Io(e) => write!(f, "I/O error: {e}"),
            AppError::Json(e) => write!(f, "JSON error: {e}"),
            AppError::Lean(msg) => write!(f, "Lean verification error: {msg}"),
            AppError::Protocol(msg) => write!(f, "Protocol violation: {msg}"),
            AppError::Internal(msg) => write!(f, "Internal server error: {msg}"),
        }
    }
}

impl std::error::Error for AppError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            AppError::Io(e) => Some(e),
            AppError::Json(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::Io(e)
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        AppError::Json(e)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            AppError::Protocol(msg) => (StatusCode::BAD_REQUEST, "bad_request", msg),
            AppError::Json(e) => (StatusCode::BAD_REQUEST, "bad_request", e.to_string()),
            AppError::Lean(msg) => {
                if msg.contains("TooLarge") || msg.contains("too large") {
                    (StatusCode::PAYLOAD_TOO_LARGE, "payload_too_large", msg)
                } else {
                    (StatusCode::INTERNAL_SERVER_ERROR, "internal", msg)
                }
            }
            AppError::Io(e) => (StatusCode::INTERNAL_SERVER_ERROR, "internal", e.to_string()),
            AppError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, "internal", msg),
        };

        let body = Json(json!({
            "error": {
                "code": code,
                "message": message
            }
        }));

        (status, body).into_response()
    }
}
