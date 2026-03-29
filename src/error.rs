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

    // Email verification errors
    #[error("Invalid verification token")]
    TokenInvalid,

    #[error("Verification token has expired")]
    TokenExpired,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_code, error_message) = match self {
            AppError::DatabaseError(ref e) => {
                // Check for unique constraint violation (PostgreSQL error code 23505)
                if let Some(db_err) = e.as_database_error()
                    && db_err.code() == Some(std::borrow::Cow::Borrowed("23505"))
                {
                    (
                        StatusCode::CONFLICT,
                        "EMAIL_ALREADY_EXISTS",
                        "Email already exists".to_string(),
                    )
                } else {
                    tracing::error!("Database error: {:?}", e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "DATABASE_ERROR",
                        "Internal database error".to_string(),
                    )
                }
            }
            AppError::ValidationError(msg) => {
                (StatusCode::UNPROCESSABLE_ENTITY, "VALIDATION_ERROR", msg)
            }
            AppError::Conflict(msg) => (StatusCode::CONFLICT, "CONFLICT", msg),
            AppError::HashError(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "HASH_ERROR",
                "Password hashing failed".to_string(),
            ),
            AppError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, "UNAUTHORIZED", msg),
            AppError::Forbidden(msg) => (StatusCode::FORBIDDEN, "FORBIDDEN", msg),
            AppError::InternalServerError => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL_ERROR",
                "Internal server error".to_string(),
            ),
            AppError::TokenInvalid => (
                StatusCode::BAD_REQUEST,
                "TOKEN_INVALID",
                "Invalid or expired verification token".to_string(),
            ),
            AppError::TokenExpired => (
                StatusCode::GONE,
                "TOKEN_EXPIRED",
                "Verification token has expired".to_string(),
            ),
        };

        let body = Json(json!({
            "err": error_code,
            "message": error_message,
        }));

        (status, body).into_response()
    }
}
