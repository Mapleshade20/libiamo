use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LanguageCode {
    En,
    Es,
    Fr,
}

impl std::str::FromStr for LanguageCode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "en" => Ok(LanguageCode::En),
            "es" => Ok(LanguageCode::Es),
            "fr" => Ok(LanguageCode::Fr),
            _ => Err(format!("Invalid language code: {}", s)),
        }
    }
}
