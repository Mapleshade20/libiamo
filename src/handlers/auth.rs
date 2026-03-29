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

    // Check if user already exists
    let existing_user = sqlx::query!(
        r#"
        SELECT is_verified, created_at,
               (now() - created_at < interval '20 minutes') as "is_too_soon!"
        FROM users
        WHERE email = $1
        "#,
        payload.email
    )
    .fetch_optional(&pool)
    .await?;

    if let Some(user) = existing_user {
        if user.is_verified {
            return Err(AppError::Conflict("Email already exists".to_string()));
        }

        if user.is_too_soon {
            return Err(AppError::TooManyRequests(
                "Please wait 20 minutes before trying to sign up again".to_string(),
            ));
        }

        // If it's been more than 20 minutes and not verified, delete the old user
        // so we can re-insert them
        sqlx::query!("DELETE FROM users WHERE email = $1", payload.email)
            .execute(&pool)
            .await?;
    }

    let first_language = payload.target_languages.first().unwrap(); // Safe because validation ensures it's not empty

    let mut tx = pool.begin().await?;
    let created_at = sqlx::query!(
        r#"
        INSERT INTO users (
            email, password_hash,
            active_language, native_language, timezone
        )
        VALUES ($1, $2, $3::language_code, $4::native_language_code, $5)
        RETURNING id, created_at
        "#,
        payload.email,
        password_hash,
        first_language as _,
        payload.native_language as _,
        payload.timezone.as_deref().unwrap_or("UTC")
    )
    .fetch_one(tx.as_mut())
    .await?;

    let user_id = created_at.id;
    let created_at = created_at.created_at;

    // Create user_learning_profiles for each target language
    for lang in &payload.target_languages {
        sqlx::query!(
            r#"
            INSERT INTO user_learning_profiles (user_id, language)
            VALUES ($1, $2::language_code)
            "#,
            user_id,
            lang as _
        )
        .execute(tx.as_mut())
        .await?;
    }

    tx.commit().await?;

    Ok((
        StatusCode::CREATED,
        Json(RegisterResponse {
            email: payload.email,
            target_languages: payload.target_languages,
            native_language: payload.native_language,
            created_at: created_at.to_rfc3339(),
        }),
    ))
}
