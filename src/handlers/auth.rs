use argon2::{
    Argon2,
    password_hash::{PasswordHasher, SaltString, rand_core::OsRng},
};
use axum::{Json, extract::State, http::StatusCode};
use sqlx::PgPool;
use validator::Validate;

use crate::error::AppError;
use crate::models::auth::{RegisterRequest, RegisterResponse};

pub async fn register(
    State(pool): State<PgPool>,
    Json(payload): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<RegisterResponse>), AppError> {
    // Validate the input
    payload
        .validate()
        .map_err(|e| AppError::ValidationError(e.to_string()))?;

    // Hash the password
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(payload.password.as_bytes(), &salt)
        .map_err(|e| AppError::HashError(e.to_string()))?
        .to_string();

    // TODO: Replace query_unchecked! with query! after setting up SQLx offline mode
    // Issue: SQLx compile-time type checking requires database connection during build
    // Solution: Use 'cargo sqlx prepare' to generate offline metadata, then 'SQLX_OFFLINE=true cargo build'
    // Insert the user into the database with hardcoded "learner" role
    // Admin role can only be set via database operations
    let result = sqlx::query_unchecked!(
        r#"
        INSERT INTO users (
            email, password_hash, nickname, role,
            target_language, native_language, timezone, level_self_assign
        )
        VALUES ($1, $2, $3, $4::user_role, $5::language_code, $6::native_language_code, $7, $8)
        RETURNING id
        "#,
        payload.email,
        password_hash,
        payload.nickname,
        "learner",
        payload.target_language,
        payload.native_language,
        payload.timezone.as_deref().unwrap_or("UTC"),
        payload.level_self_assign
    )
    .fetch_one(&pool)
    .await?;
    
    let user_id = result.id;

    Ok((
        StatusCode::CREATED,
        Json(RegisterResponse {
            message: "User registered successfully".to_string(),
            user_id,
        }),
    ))
}
