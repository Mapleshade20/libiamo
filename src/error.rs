use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Hash error: {0}")]
    HashError(String),

    #[error("Internal Server Error")]
    InternalServerError,

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AppError::DatabaseError(ref e) => {
                // Check for unique constraint violation (PostgreSQL error code 23505)
                if let Some(db_err) = e.as_database_error()
                    && db_err.code() == Some(std::borrow::Cow::Borrowed("23505"))
                {
                    (StatusCode::CONFLICT, "Email already exists".to_string())
                } else {
                    tracing::error!("Database error: {:?}", e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Internal database error".to_string(),
                    )
                }
            }
            AppError::ValidationError(msg) => (StatusCode::BAD_REQUEST, msg),
            AppError::Conflict(msg) => (StatusCode::CONFLICT, msg),
            AppError::HashError(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Password hashing failed".to_string(),
            ),
            AppError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg),
            AppError::Forbidden(msg) => (StatusCode::FORBIDDEN, msg),
            AppError::InternalServerError => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error".to_string(),
            ),
        };

        let body = Json(json!({
            "error": error_message,
        }));

        (status, body).into_response()
    }
}
