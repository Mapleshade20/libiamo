use crate::models::auth::EmailVerificationToken;
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use rand::RngCore;
use sha2::{Digest, Sha256};

// generate email verification token
pub fn generate_email_verification_token() -> EmailVerificationToken {
    let mut random_bytes = [0u8; 32];
    let mut rng = rand::thread_rng();
    rng.fill_bytes(&mut random_bytes);
    let token = BASE64.encode(random_bytes);

    let token_hash = hash_token(&token);

    EmailVerificationToken { token, token_hash }
}

pub fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    let result = hasher.finalize();
    hex::encode(result)
}
