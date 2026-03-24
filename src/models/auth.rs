use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct RegisterRequest {
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 8))]
    pub password: String,
    pub nickname: String,
    pub role: String,
    pub target_language: String,
    pub native_language: String,
    pub timezone: Option<String>,
    #[validate(range(min = 1, max = 5))]
    pub level_self_assign: Option<i32>,
}

#[derive(Serialize)]
pub struct RegisterResponse {
    pub message: String,
    pub user_id: i32,
}
