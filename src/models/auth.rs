use std::borrow::Cow;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::models::types::LanguageCode;

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct RegisterRequest {
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 8))]
    pub password: String,
    #[validate(custom(function = "validate_target_languages"))]
    pub target_languages: Vec<String>,
    pub native_language: String,
    pub timezone: Option<String>,
}

fn validate_target_languages(langs: &Vec<String>) -> Result<(), validator::ValidationError> {
    if langs.is_empty() {
        return Err(validator::ValidationError::new(
            "target_languages cannot be empty",
        ));
    }
    for lang in langs {
        if let Err(e) = LanguageCode::from_str(lang) {
            let mut err = validator::ValidationError::new("unsupported_language");
            err.message = Some(Cow::from(e));
            return Err(err);
        }
    }
    Ok(())
}

#[derive(Serialize)]
pub struct RegisterResponse {
    pub email: String,
    pub target_languages: Vec<String>,
    pub native_language: String,
    pub created_at: String,
}
