use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ServiceError {
    #[error("Not found")]
    NotFound,

    #[error("Redis error: {0}")]
    Redis(#[from] redis::RedisError),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Internal error: {0}")]
    Internal(#[from] anyhow::Error),
}

impl IntoResponse for ServiceError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ServiceError::NotFound => (StatusCode::NOT_FOUND, "Not found"),
            ServiceError::Redis(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Database error"),
            ServiceError::Serialization(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Serialization error"),
            ServiceError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Internal error"),
        };

        let body = Json(json!({
            "error": message,
        }));

        (status, body).into_response()
    }
}