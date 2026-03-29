use argon2::{
    Argon2,
    password_hash::{PasswordHasher, SaltString, rand_core::OsRng},
};
use axum::{Json, extract::State, http::StatusCode};
use chrono::{Duration, Utc};
use sqlx::PgPool;
use std::env;
use tracing::info;
use validator::Validate;

use crate::error::AppError;
use crate::models::auth::{RegisterRequest, RegisterResponse, VerifyEmailRequest};
use crate::services::email::{EmailConfig, spawn_send_verification_email};
use crate::services::token;

pub async fn register(
    State(pool): State<PgPool>,
    Json(payload): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<RegisterResponse>), AppError> {
    // Validate the input per API spec:
    // - email: valid email
    // - password: 8–72 chars, at least one uppercase, one lowercase, one digit
    // - target_language: one of 'en', 'es', 'fr'
    // - native_language: BCP 47 format
    // - level_self_assign: 1-5
    payload
        .validate()
        .map_err(|e| AppError::ValidationError(e.to_string()))?;

    // Validate password complexity (at least uppercase, lowercase, digit)
    if !payload.password.chars().any(|c| c.is_uppercase()) {
        return Err(AppError::ValidationError(
            "Password must contain at least one uppercase letter".to_string(),
        ));
    }
    if !payload.password.chars().any(|c| c.is_lowercase()) {
        return Err(AppError::ValidationError(
            "Password must contain at least one lowercase letter".to_string(),
        ));
    }
    if !payload.password.chars().any(|c| c.is_numeric()) {
        return Err(AppError::ValidationError(
            "Password must contain at least one digit".to_string(),
        ));
    }

    // Hash the password
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(payload.password.as_bytes(), &salt)
        .map_err(|e| AppError::HashError(e.to_string()))?
        .to_string();

    // Start a transaction for atomicity
    let mut tx = pool.begin().await?;

    // Insert the user into the database with hardcoded "learner" role
    // Admin role can only be set via database operations
    let result = sqlx::query_unchecked!(
        r#"
        INSERT INTO users (
            email, password_hash, role,
            target_language, native_language, timezone, level_self_assign, is_verified
        )
        VALUES ($1, $2, $3::user_role, $4::language_code, $5::native_language_code, $6, $7, false)
        RETURNING id, email, created_at
        "#,
        payload.email,
        password_hash,
        "learner",
        payload.target_language,
        payload.native_language,
        payload.timezone.as_deref().unwrap_or("UTC"),
        payload.level_self_assign
    )
    .fetch_one(&mut *tx)
    .await?;

    let user_id = result.id;

    // Generate email verification token
    let email_token = token::generate_email_verification_token();
    let token_expiration_hours = env::var("TOKEN_EXPIRATION_HOURS")
        .unwrap_or_else(|_| "24".to_string())
        .parse::<i64>()
        .unwrap_or(24);
    let expires_at = Utc::now() + Duration::hours(token_expiration_hours);

    // Store the token hash in the database
    sqlx::query_unchecked!(
        r#"
        INSERT INTO auth_tokens (user_id, token_hash, purpose, expires_at)
        VALUES ($1, $2, $3::token_purpose, $4)
        "#,
        user_id,
        email_token.token_hash,
        "email_verification",
        expires_at
    )
    .execute(&mut *tx)
    .await?;

    // Commit the transaction
    tx.commit().await?;

    // Asynchronously send verification email
    let email_config = EmailConfig::from_env().unwrap_or_else(|_| {
        // Development fallback configuration
        EmailConfig {
            smtp_host: "localhost".to_string(),
            smtp_port: 587,
            smtp_username: String::new(),
            smtp_password: String::new(),
            from_email: "noreply@libiamo.com".to_string(),
            frontend_url: "http://localhost:5173".to_string(),
        }
    });

    spawn_send_verification_email(
        payload.email.clone(),
        email_token.token.clone(),
        email_config,
    );

    info!(
        "User registered successfully with ID: {}, email verification email sent",
        user_id
    );

    Ok((
        StatusCode::CREATED,
        Json(RegisterResponse {
            id: user_id,
            email: result.email,
            role: "learner".to_string(),
            target_language: payload.target_language,
            native_language: payload.native_language,
            created_at: result.created_at,
        }),
    ))
}

pub async fn verify_email(
    State(pool): State<PgPool>,
    Json(payload): Json<VerifyEmailRequest>,
) -> Result<StatusCode, AppError> {
    // Compute the hash of the provided token
    let token_hash = token::hash_token(&payload.token);

    // Start a transaction
    let mut tx = pool.begin().await?;

    // Find the token in the database
    let token_record = sqlx::query_unchecked!(
        r#"
        SELECT id, user_id, used_at, expires_at
        FROM auth_tokens
        WHERE token_hash = $1 AND purpose = $2::token_purpose
        "#,
        token_hash,
        "email_verification"
    )
    .fetch_optional(&mut *tx)
    .await?;

    let token_record = token_record.ok_or(AppError::TokenInvalid)?;

    // Check if token has already been used
    if token_record.used_at.is_some() {
        return Err(AppError::TokenInvalid);
    }

    // Check if token has expired
    if Utc::now() > token_record.expires_at {
        return Err(AppError::TokenExpired);
    }

    // Mark the token as used and verify the user
    sqlx::query_unchecked!(
        r#"
        UPDATE auth_tokens
        SET used_at = $1
        WHERE id = $2
        "#,
        Utc::now(),
        token_record.id
    )
    .execute(&mut *tx)
    .await?;

    // Update the user's is_verified flag
    sqlx::query_unchecked!(
        r#"
        UPDATE users
        SET is_verified = true
        WHERE id = $1
        "#,
        token_record.user_id
    )
    .execute(&mut *tx)
    .await?;

    // Commit the transaction
    tx.commit().await?;

    info!("User {} verified email successfully", token_record.user_id);

    // Return 204 No Content as per API spec
    Ok(StatusCode::NO_CONTENT)
}
