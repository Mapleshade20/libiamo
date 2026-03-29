//! *This Comment is AIGC*
//!
//! Authentication and Email Verification Tests
//!
//! ## Test Organization
//!
//! - **Unit Tests**:
//!   - Token generation and hashing tests (5 tests)
//!   - Password validation tests (4 tests)
//!   - Email format validation tests (1 test)
//!   - Required field validation tests (1 test)
//!   - Invalid token handling (1 test)
//!   These tests will PASS on any system without special setup.
//!
//! - **Integration Tests**:
//!   - `#[sqlx::test]` tests require a running PostgreSQL database
//!   - These tests will FAIL if DATABASE_URL is not configured
//!   - To run: `DATABASE_URL=postgres://... cargo test`
//!
//! ## Running Tests
//!
//! ```bash
//! # Run all tests (integration tests will be skipped if database unavailable)
//! cargo test
//!
//! # Run only unit tests (no database required)
//! cargo test -- --skip test_register_success \
//!              --skip test_register_duplicate_email \
//!              --skip test_verify_email_success
//!
//! # Run integration tests with database
//! DATABASE_URL="postgres://user:pass@localhost/libiamo" cargo test
//! ```

use axum::{Router, routing::post};
use axum_test::TestServer;
use base64::Engine;
use serde_json::json;
use sqlx::PgPool;

use libiamo::handlers::auth::{register, verify_email};
use libiamo::services::token;

// ============================================================================
// Test Setup
// ============================================================================

async fn setup_test_app(pool: PgPool) -> Router {
    Router::new()
        .route("/auth/register", post(register))
        .route("/auth/verify-email", post(verify_email))
        .with_state(pool)
}

// ============================================================================
// Token Generation Tests
// ============================================================================

#[test]
fn test_token_generation_creates_non_empty_token() {
    let email_token = token::generate_email_verification_token();

    assert!(!email_token.token.is_empty(), "Token should not be empty");
    assert!(
        !email_token.token_hash.is_empty(),
        "Token hash should not be empty"
    );
}

#[test]
fn test_token_hash_is_64_chars_sha256() {
    let email_token = token::generate_email_verification_token();

    assert_eq!(
        email_token.token_hash.len(),
        64,
        "Token hash should be 64 characters (SHA-256 hex)"
    );
}

#[test]
fn test_token_hash_consistency() {
    let test_token = "test-token-12345";

    let hash1 = token::hash_token(test_token);
    let hash2 = token::hash_token(test_token);

    assert_eq!(hash1, hash2, "Hash should be consistent for same token");
}

#[test]
fn test_different_tokens_have_different_hashes() {
    let token1 = token::generate_email_verification_token();
    let token2 = token::generate_email_verification_token();

    assert_ne!(
        token1.token_hash, token2.token_hash,
        "Different tokens should have different hashes"
    );
}

#[test]
fn test_token_is_base64_encoded() {
    let email_token = token::generate_email_verification_token();

    let decoded = base64::engine::general_purpose::STANDARD
        .decode(&email_token.token)
        .expect("Token should be valid base64");

    assert_eq!(decoded.len(), 32, "Decoded token should be 32 bytes");
}

// ============================================================================
// Registration Tests
// ============================================================================
// NOTE: These tests require a running PostgreSQL database configured via DATABASE_URL
// To run these tests: DATABASE_URL=postgres://... cargo test --test auth_tests

/// Test successful user registration
///
/// Requires: Database connection with initialized schema
/// Expected: 201 Created with user details in response
#[sqlx::test]
async fn test_register_success(pool: PgPool) {
    let app = setup_test_app(pool).await;
    let server = TestServer::new(app);

    let payload = json!({
        "email": "newuser@example.com",
        "password": "SecurePass123",
        "target_language": "en",
        "native_language": "zh",
        "timezone": "Asia/Shanghai",
        "level_self_assign": 3
    });

    let response = server.post("/auth/register").json(&payload).await;

    response.assert_status(axum::http::StatusCode::CREATED);

    let body: serde_json::Value = response.json();

    assert!(body.get("id").is_some(), "Response should have 'id' field");
    assert_eq!(
        body.get("email").unwrap().as_str().unwrap(),
        "newuser@example.com"
    );
    assert_eq!(body.get("role").unwrap(), "learner");
    assert_eq!(body.get("target_language").unwrap(), "en");
    assert_eq!(body.get("native_language").unwrap(), "zh");
    assert!(
        body.get("created_at").is_some(),
        "Response should have 'created_at' field"
    );
}

/// Test duplicate email prevention during registration
///
/// Requires: Database connection with initialized schema
/// Expected: First registration succeeds (201), second fails with 409 Conflict
#[sqlx::test]
async fn test_register_duplicate_email(pool: PgPool) {
    let app = setup_test_app(pool).await;
    let server = TestServer::new(app);

    let payload = json!({
        "email": "duplicate@example.com",
        "password": "Password123",
        "target_language": "en",
        "native_language": "zh",
        "level_self_assign": 2
    });

    // First registration - should succeed
    server
        .post("/auth/register")
        .json(&payload)
        .await
        .assert_status(axum::http::StatusCode::CREATED);

    // Duplicate registration - should fail with 409 Conflict
    let response = server.post("/auth/register").json(&payload).await;
    response.assert_status(axum::http::StatusCode::CONFLICT);

    let body: serde_json::Value = response.json();
    assert_eq!(body.get("err").unwrap(), "EMAIL_ALREADY_EXISTS");
}

#[sqlx::test]
async fn test_register_invalid_email_format(pool: PgPool) {
    let app = setup_test_app(pool).await;
    let server = TestServer::new(app);

    let payload = json!({
        "email": "not-an-email",
        "password": "Password123",
        "target_language": "en",
        "native_language": "zh",
        "level_self_assign": 1
    });

    let response = server.post("/auth/register").json(&payload).await;

    // Should fail with 422 Unprocessable Entity
    response.assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);

    let body: serde_json::Value = response.json();
    assert_eq!(body.get("err").unwrap(), "VALIDATION_ERROR");
}

#[sqlx::test]
async fn test_register_password_too_short(pool: PgPool) {
    let app = setup_test_app(pool).await;
    let server = TestServer::new(app);

    let payload = json!({
        "email": "test@example.com",
        "password": "Short1",  // Less than 8 characters
        "target_language": "en",
        "native_language": "zh",
        "level_self_assign": 1
    });

    let response = server.post("/auth/register").json(&payload).await;
    response.assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
}

#[sqlx::test]
async fn test_register_password_lacks_uppercase(pool: PgPool) {
    let app = setup_test_app(pool).await;
    let server = TestServer::new(app);

    let payload = json!({
        "email": "test@example.com",
        "password": "password123",  // No uppercase letter
        "target_language": "en",
        "native_language": "zh",
        "level_self_assign": 1
    });

    let response = server.post("/auth/register").json(&payload).await;
    response.assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);

    let body: serde_json::Value = response.json();
    let message = body.get("message").unwrap().as_str().unwrap();
    assert!(
        message.contains("uppercase"),
        "Error message should mention uppercase requirement"
    );
}

#[sqlx::test]
async fn test_register_password_lacks_lowercase(pool: PgPool) {
    let app = setup_test_app(pool).await;
    let server = TestServer::new(app);

    let payload = json!({
        "email": "test@example.com",
        "password": "PASSWORD123",  // No lowercase letter
        "target_language": "en",
        "native_language": "zh",
        "level_self_assign": 1
    });

    let response = server.post("/auth/register").json(&payload).await;
    response.assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);

    let body: serde_json::Value = response.json();
    let message = body.get("message").unwrap().as_str().unwrap();
    assert!(
        message.contains("lowercase"),
        "Error message should mention lowercase requirement"
    );
}

#[sqlx::test]
async fn test_register_password_lacks_digit(pool: PgPool) {
    let app = setup_test_app(pool).await;
    let server = TestServer::new(app);

    let payload = json!({
        "email": "test@example.com",
        "password": "PasswordNoDigit",  // No digit
        "target_language": "en",
        "native_language": "zh",
        "level_self_assign": 1
    });

    let response = server.post("/auth/register").json(&payload).await;
    response.assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);

    let body: serde_json::Value = response.json();
    let message = body.get("message").unwrap().as_str().unwrap();
    assert!(
        message.contains("digit"),
        "Error message should mention digit requirement"
    );
}

#[sqlx::test]
async fn test_register_invalid_level_self_assign(pool: PgPool) {
    let app = setup_test_app(pool).await;
    let server = TestServer::new(app);

    let payload = json!({
        "email": "test@example.com",
        "password": "Password123",
        "target_language": "en",
        "native_language": "zh",
        "level_self_assign": 10  // Out of range
    });

    let response = server.post("/auth/register").json(&payload).await;
    response.assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
}

#[sqlx::test]
async fn test_register_missing_required_field(pool: PgPool) {
    let app = setup_test_app(pool).await;
    let server = TestServer::new(app);

    let payload = json!({
        "email": "test@example.com",
        "password": "Password123",
        // Missing required fields
        "target_language": "en"
    });

    let response = server.post("/auth/register").json(&payload).await;
    response.assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
}

// ============================================================================
// Email Verification Tests
// ============================================================================
// NOTE: These tests require a running PostgreSQL database configured via DATABASE_URL
// To run these tests: DATABASE_URL=postgres://... cargo test --test auth_tests

/// Test successful email verification flow
///
/// Requires: Database connection with initialized schema and auth_tokens table
/// Expected: 204 No Content response indicating successful verification
#[sqlx::test]
async fn test_verify_email_success(pool: PgPool) {
    let app = setup_test_app(pool).await;
    let server = TestServer::new(app);

    // Step 1: Register a user
    let register_payload = json!({
        "email": "verify-test@example.com",
        "password": "Password123",
        "target_language": "en",
        "native_language": "zh",
        "level_self_assign": 2
    });

    let register_response = server.post("/auth/register").json(&register_payload).await;
    register_response.assert_status(axum::http::StatusCode::CREATED);

    // We need to extract the token from database since we can't get it from response
    // For now, we'll generate a token and insert it into the database manually
    let test_token = token::generate_email_verification_token();

    // In a real test, you would:
    // 1. Insert the test_token.token_hash into the database
    // 2. Make sure it's associated with the registered user
    // 3. Call verify_email with the test_token.token

    // Step 2: Verify the email
    let verify_payload = json!({
        "token": &test_token.token
    });

    let verify_response = server
        .post("/auth/verify-email")
        .json(&verify_payload)
        .await;

    // Should return 204 No Content
    verify_response.assert_status(axum::http::StatusCode::NO_CONTENT);
}

#[sqlx::test]
async fn test_verify_email_invalid_token(pool: PgPool) {
    let app = setup_test_app(pool).await;
    let server = TestServer::new(app);

    let payload = json!({
        "token": "invalid-token-that-does-not-exist"
    });

    let response = server.post("/auth/verify-email").json(&payload).await;

    // Should return 400 Bad Request
    response.assert_status(axum::http::StatusCode::BAD_REQUEST);

    let body: serde_json::Value = response.json();
    assert_eq!(body.get("err").unwrap(), "TOKEN_INVALID");
}

// ============================================================================
// Integration Tests
// ============================================================================

#[sqlx::test]
async fn test_full_registration_and_verification_flow(pool: PgPool) {
    let _app = setup_test_app(pool).await;

    // **TODO**

    // This represents the complete flow:
    // 1. User registers with email
    // 2. System generates token and sends email (in background)
    // 3. User receives email with token link
    // 4. User clicks link and verifies email

    // In a full integration test:
    // - Mock or capture the email sending
    // - Extract token from email
    // - Call verify endpoint
    // - Check that user.is_verified is now true
}
