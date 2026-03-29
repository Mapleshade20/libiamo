//! Registration endpoint tests
//!
//! These are integration tests that require a running PostgreSQL database.
//! Run with: `DATABASE_URL=postgres://... cargo test register_tests`

use axum_test::TestServer;
use serde_json::json;
use sqlx::PgPool;

use super::common::setup_test_app;

// ============================================================================
// Registration Tests
// ============================================================================

/// Test successful user registration
///
/// Requires: Database connection with initialized schema
/// Expected: 201 Created with user details in response
#[sqlx::test]
async fn test_register_success(pool: PgPool) {
    let (app, _pool) = setup_test_app(pool).await;
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
    let (app, _pool) = setup_test_app(pool).await;
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

/// Test that invalid email format is rejected
#[sqlx::test]
async fn test_register_invalid_email_format(pool: PgPool) {
    let (app, _pool) = setup_test_app(pool).await;
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

/// Test that password shorter than 8 characters is rejected
#[sqlx::test]
async fn test_register_password_too_short(pool: PgPool) {
    let (app, _pool) = setup_test_app(pool).await;
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

/// Test that password without uppercase letter is rejected
#[sqlx::test]
async fn test_register_password_lacks_uppercase(pool: PgPool) {
    let (app, _pool) = setup_test_app(pool).await;
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

/// Test that password without lowercase letter is rejected
#[sqlx::test]
async fn test_register_password_lacks_lowercase(pool: PgPool) {
    let (app, _pool) = setup_test_app(pool).await;
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

/// Test that password without digit is rejected
#[sqlx::test]
async fn test_register_password_lacks_digit(pool: PgPool) {
    let (app, _pool) = setup_test_app(pool).await;
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

/// Test that invalid level_self_assign (out of range) is rejected
#[sqlx::test]
async fn test_register_invalid_level_self_assign(pool: PgPool) {
    let (app, _pool) = setup_test_app(pool).await;
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

/// Test that missing required fields are rejected
#[sqlx::test]
async fn test_register_missing_required_field(pool: PgPool) {
    let (app, _pool) = setup_test_app(pool).await;
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
