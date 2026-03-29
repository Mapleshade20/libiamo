use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct RegisterRequest {
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 8, max = 72))]
    pub password: String,
    pub target_language: String,
    pub native_language: String,
    pub timezone: Option<String>,
    #[validate(range(min = 1, max = 5))]
    pub level_self_assign: i32,
}

#[derive(Debug, Serialize)]
pub struct RegisterResponse {
    pub id: i32,
    pub email: String,
    pub role: String,
    pub target_language: String,
    pub native_language: String,
    pub created_at: DateTime<Utc>,
}

// Email verification models
#[derive(Debug, Serialize, Deserialize)]
pub struct VerifyEmailRequest {
    pub token: String,
}

#[derive(Debug)]
pub struct EmailVerificationToken {
    pub token: String,
    pub token_hash: String,
}

#[derive(Debug, Serialize)]
pub struct VerificationStatus {
    pub email: String,
    pub is_verified: bool,
    pub created_at: DateTime<Utc>,
}
