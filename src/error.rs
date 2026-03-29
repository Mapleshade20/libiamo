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
        // Handle dynamic error codes (Unauthorized, Forbidden use the message as the error code)
        match self {
            AppError::Unauthorized(msg) => {
                let body = Json(json!({
                    "err": msg,
                    "message": msg,
                }));
                (StatusCode::UNAUTHORIZED, body).into_response()
            }
            AppError::Forbidden(msg) => {
                let body = Json(json!({
                    "err": msg,
                    "message": msg,
                }));
                (StatusCode::FORBIDDEN, body).into_response()
            }
            // Handle all other errors with static error codes
            err => {
                let (status, error_code, error_message) = match err {
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
                            let error_detail = if let Some(db_err) = e.as_database_error() {
                                format!("PG Code: {:?}, Message: {}", db_err.code(), db_err.message())
                            } else {
                                format!("{:?}", e)
                            };
                            tracing::error!("Database error: {}", error_detail);
                            (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                "DATABASE_ERROR",
                                format!("Internal database error: {}", error_detail),
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
                    // These should be handled above, but included for completeness
                    AppError::Unauthorized(_) | AppError::Forbidden(_) => unreachable!(),
                };

                let body = Json(json!({
                    "err": error_code,
                    "message": error_message,
                }));

                (status, body).into_response()
            }
        }
    }
}
