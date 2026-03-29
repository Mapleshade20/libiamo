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

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub id: i32,
    pub email: String,
    pub role: String,
    pub nickname: Option<String>,
    pub avatar_url: Option<String>,
    pub target_language: String,
    pub native_language: String,
    pub timezone: String,
    pub gems_balance: i32,
    pub level_self_assign: Option<i32>,
}

#[derive(Debug)]
pub struct UserLoginRecord {
    pub id: i32,
    pub email: String,
    pub password_hash: String,
    pub is_verified: bool,
    pub role: String,
    pub nickname: Option<String>,
    pub avatar_url: Option<String>,
    pub target_language: String,
    pub native_language: String,
    pub timezone: String,
    pub gems_balance: i32,
    pub level_self_assign: Option<i32>,
}
