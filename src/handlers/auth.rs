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

    // Insert the user into the database
    let user_id = sqlx::query!(
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
        payload.role as _,
        payload.target_language as _,
        payload.native_language as _,
        payload.timezone.as_deref().unwrap_or("UTC"),
        payload.level_self_assign
    )
    .fetch_one(&pool)
    .await?
    .id;

    Ok((
        StatusCode::CREATED,
        Json(RegisterResponse {
            message: "User registered successfully".to_string(),
            user_id,
        }),
    ))
}
