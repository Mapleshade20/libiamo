//! Login endpoint tests
//!
//! These are integration tests that require a running PostgreSQL database.
//! Run with: `DATABASE_URL=postgres://... cargo test login_tests`

use axum_test::TestServer;
use serde_json::json;
use sqlx::PgPool;

use super::common::setup_test_app;
use libiamo::services::token;

// ============================================================================
// Login Tests
// ============================================================================

/// Test successful login with correct credentials
///
/// Requires: Database connection with initialized schema
/// Expected: 200 OK with user profile and Set-Cookie header
#[sqlx::test]
async fn test_login_success(pool: PgPool) {
    let (app, pool) = setup_test_app(pool).await;
    let server = TestServer::new(app);

    // Step 1: Register a user
    let register_payload = json!({
        "email": "login-test@example.com",
        "password": "LoginPass123",
        "target_language": "en",
        "native_language": "zh-CN",
        "timezone": "Asia/Shanghai",
        "level_self_assign": 3
    });

    server
        .post("/auth/register")
        .json(&register_payload)
        .await
        .assert_status(axum::http::StatusCode::CREATED);

    // Step 2: Verify email (manually insert token status for this test)
    let _test_token = token::generate_email_verification_token();
    sqlx::query!(
        r#"
        UPDATE auth_tokens
        SET used_at = NOW()
        WHERE user_id = (SELECT id FROM users WHERE email = $1)
        "#,
        "login-test@example.com"
    )
    .execute(&pool)
    .await
    .ok();

    sqlx::query!(
        r#"
        UPDATE users
        SET is_verified = true
        WHERE email = $1
        "#,
        "login-test@example.com"
    )
    .execute(&pool)
    .await
    .ok();

    // Step 3: Login with correct credentials
    let login_payload = json!({
        "email": "login-test@example.com",
        "password": "LoginPass123"
    });

    let response = server.post("/auth/login").json(&login_payload).await;

    response.assert_status(axum::http::StatusCode::OK);

    let body: serde_json::Value = response.json();

    // Verify response contains all required fields
    assert!(body.get("id").is_some(), "Response should have 'id' field");
    assert_eq!(
        body.get("email").unwrap().as_str().unwrap(),
        "login-test@example.com"
    );
    assert_eq!(body.get("role").unwrap(), "learner");
    assert_eq!(body.get("target_language").unwrap(), "en");
    assert_eq!(body.get("native_language").unwrap(), "zh-CN");
    assert_eq!(body.get("timezone").unwrap(), "Asia/Shanghai");
    assert!(
        body.get("gems_balance").is_some(),
        "Response should have 'gems_balance' field"
    );
    assert_eq!(body.get("level_self_assign").unwrap(), 3);

    // Verify Set-Cookie header is present
    let cookie_header = response.headers().get("set-cookie");
    assert!(
        cookie_header.is_some(),
        "Response should have Set-Cookie header"
    );

    let cookie_str = cookie_header.unwrap().to_str().unwrap();
    assert!(
        cookie_str.contains("libiamo_session"),
        "Cookie should be named 'libiamo_session'"
    );
    assert!(
        cookie_str.contains("HttpOnly"),
        "Cookie should have HttpOnly flag"
    );
    assert!(
        cookie_str.contains("SameSite=Lax"),
        "Cookie should have SameSite=Lax"
    );
}

/// Test login with invalid credentials (wrong email)
///
/// Expected: 401 Unauthorized with INVALID_CREDENTIALS error
#[sqlx::test]
async fn test_login_invalid_email(pool: PgPool) {
    let (app, _pool) = setup_test_app(pool).await;
    let server = TestServer::new(app);

    let payload = json!({
        "email": "nonexistent@example.com",
        "password": "SomePassword123"
    });

    let response = server.post("/auth/login").json(&payload).await;

    response.assert_status(axum::http::StatusCode::UNAUTHORIZED);

    let body: serde_json::Value = response.json();
    assert_eq!(body.get("err").unwrap(), "INVALID_CREDENTIALS");
}

/// Test login with invalid credentials (wrong password)
///
/// Expected: 401 Unauthorized with INVALID_CREDENTIALS error
#[sqlx::test]
async fn test_login_invalid_password(pool: PgPool) {
    let (app, pool) = setup_test_app(pool).await;
    let server = TestServer::new(app);

    // First, register a verified user
    let register_payload = json!({
        "email": "wrong-pass-test@example.com",
        "password": "CorrectPass123",
        "target_language": "en",
        "native_language": "zh",
        "level_self_assign": 2
    });

    server
        .post("/auth/register")
        .json(&register_payload)
        .await
        .assert_status(axum::http::StatusCode::CREATED);

    // Mark user as verified
    sqlx::query!(
        r#"
        UPDATE users
        SET is_verified = true
        WHERE email = $1
        "#,
        "wrong-pass-test@example.com"
    )
    .execute(&pool)
    .await
    .ok();

    // Try to login with wrong password
    let login_payload = json!({
        "email": "wrong-pass-test@example.com",
        "password": "WrongPassword123"
    });

    let response = server.post("/auth/login").json(&login_payload).await;

    response.assert_status(axum::http::StatusCode::UNAUTHORIZED);

    let body: serde_json::Value = response.json();
    assert_eq!(body.get("err").unwrap(), "INVALID_CREDENTIALS");
}

/// Test login with unverified email
///
/// Expected: 403 Forbidden with EMAIL_NOT_VERIFIED error
#[sqlx::test]
async fn test_login_email_not_verified(pool: PgPool) {
    let (app, _pool) = setup_test_app(pool).await;
    let server = TestServer::new(app);

    // Register a user
    let register_payload = json!({
        "email": "unverified-test@example.com",
        "password": "Password123",
        "target_language": "en",
        "native_language": "zh",
        "level_self_assign": 2
    });

    server
        .post("/auth/register")
        .json(&register_payload)
        .await
        .assert_status(axum::http::StatusCode::CREATED);

    // User is registered but NOT verified - try to login
    let login_payload = json!({
        "email": "unverified-test@example.com",
        "password": "Password123"
    });

    let response = server.post("/auth/login").json(&login_payload).await;

    // Should fail with 403 Forbidden
    response.assert_status(axum::http::StatusCode::FORBIDDEN);

    let body: serde_json::Value = response.json();
    assert_eq!(body.get("err").unwrap(), "EMAIL_NOT_VERIFIED");
}

/// Test login with missing email field
///
/// Expected: 422 Unprocessable Entity
#[sqlx::test]
async fn test_login_missing_email(pool: PgPool) {
    let (app, _pool) = setup_test_app(pool).await;
    let server = TestServer::new(app);

    let payload = json!({
        "password": "Password123"
        // Missing email field
    });

    let response = server.post("/auth/login").json(&payload).await;

    // Should fail with 422 Unprocessable Entity
    response.assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
}

/// Test login with missing password field
///
/// Expected: 422 Unprocessable Entity
#[sqlx::test]
async fn test_login_missing_password(pool: PgPool) {
    let (app, _pool) = setup_test_app(pool).await;
    let server = TestServer::new(app);

    let payload = json!({
        "email": "test@example.com"
        // Missing password field
    });

    let response = server.post("/auth/login").json(&payload).await;

    // Should fail with 422 Unprocessable Entity
    response.assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
}

/// Test login with empty email string
///
/// Expected: 422 Unprocessable Entity with VALIDATION_ERROR
#[sqlx::test]
async fn test_login_empty_email(pool: PgPool) {
    let (app, _pool) = setup_test_app(pool).await;
    let server = TestServer::new(app);

    let payload = json!({
        "email": "",
        "password": "Password123"
    });

    let response = server.post("/auth/login").json(&payload).await;

    response.assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);

    let body: serde_json::Value = response.json();
    assert_eq!(body.get("err").unwrap(), "VALIDATION_ERROR");
}

/// Test login with empty password string
///
/// Expected: 422 Unprocessable Entity with VALIDATION_ERROR
#[sqlx::test]
async fn test_login_empty_password(pool: PgPool) {
    let (app, _pool) = setup_test_app(pool).await;
    let server = TestServer::new(app);

    let payload = json!({
        "email": "test@example.com",
        "password": ""
    });

    let response = server.post("/auth/login").json(&payload).await;

    response.assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);

    let body: serde_json::Value = response.json();
    assert_eq!(body.get("err").unwrap(), "VALIDATION_ERROR");
}
