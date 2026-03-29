//! Token generation and hashing tests
//!
//! These are unit tests that don't require a database.
//! Run with: `cargo test token_tests`

use base64::Engine;
use libiamo::services::token;

// ============================================================================
// Token Generation Tests
// ============================================================================

/// Test that email verification token is generated with non-empty values
#[test]
fn test_token_generation_creates_non_empty_token() {
    let email_token = token::generate_email_verification_token();

    assert!(!email_token.token.is_empty(), "Token should not be empty");
    assert!(
        !email_token.token_hash.is_empty(),
        "Token hash should not be empty"
    );
}

/// Test that token hash is exactly 64 characters (SHA-256 hex)
#[test]
fn test_token_hash_is_64_chars_sha256() {
    let email_token = token::generate_email_verification_token();

    assert_eq!(
        email_token.token_hash.len(),
        64,
        "Token hash should be 64 characters (SHA-256 hex)"
    );
}

/// Test that hashing the same token produces consistent results
#[test]
fn test_token_hash_consistency() {
    let test_token = "test-token-12345";

    let hash1 = token::hash_token(test_token);
    let hash2 = token::hash_token(test_token);

    assert_eq!(hash1, hash2, "Hash should be consistent for same token");
}

/// Test that different tokens produce different hashes
#[test]
fn test_different_tokens_have_different_hashes() {
    let token1 = token::generate_email_verification_token();
    let token2 = token::generate_email_verification_token();

    assert_ne!(
        token1.token_hash, token2.token_hash,
        "Different tokens should have different hashes"
    );
}

/// Test that generated tokens are valid base64 encoded
#[test]
fn test_token_is_base64_encoded() {
    let email_token = token::generate_email_verification_token();

    let decoded = base64::engine::general_purpose::STANDARD
        .decode(&email_token.token)
        .expect("Token should be valid base64");

    assert_eq!(decoded.len(), 32, "Decoded token should be 32 bytes");
}
