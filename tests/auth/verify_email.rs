//! Email verification endpoint tests
//!
//! These are integration tests that require a running PostgreSQL database.
//! Run with: `DATABASE_URL=postgres://... cargo test verify_email_tests`

use axum_test::TestServer;
use serde_json::json;
use sqlx::PgPool;
use chrono::{Utc, Duration};

use super::common::setup_test_app;
use libiamo::services::token;

// ============================================================================
// Email Verification Tests
// ============================================================================

/// Test successful email verification flow
///
/// Requires: Database connection with initialized schema and auth_tokens table
/// Expected: 204 No Content response indicating successful verification
#[sqlx::test]
async fn test_verify_email_success(pool: PgPool) {
    let (app, pool) = setup_test_app(pool).await;
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

    // Step 2: Get the user ID and insert a valid verification token
    let user_result = sqlx::query!(
        r#"SELECT id FROM users WHERE email = $1"#,
        "verify-test@example.com"
    )
    .fetch_one(&pool)
    .await
    .expect("Unable to fetch user");

    let test_token = token::generate_email_verification_token();
    let expires_at = Utc::now() + Duration::hours(24);

    // Insert the token into the database
    sqlx::query!(
        r#"
        INSERT INTO auth_tokens (user_id, token_hash, purpose, expires_at)
        VALUES ($1, $2, 'email_verification'::token_purpose, $3)
        "#,
        user_result.id,
        test_token.token_hash,
        expires_at
    )
    .execute(&pool)
    .await
    .expect("Unable to insert token");

    // Step 3: Verify the email with the inserted token
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

/// Test that invalid verification token is rejected
///
/// Expected: 400 Bad Request with TOKEN_INVALID error
#[sqlx::test]
async fn test_verify_email_invalid_token(pool: PgPool) {
    let (app, _pool) = setup_test_app(pool).await;
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
