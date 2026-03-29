//! Logout endpoint tests
//!
//! These are integration tests that require a running PostgreSQL database.
//! Run with: `DATABASE_URL=postgres://... cargo test logout`

use axum_test::TestServer;
use serde_json::json;
use sqlx::PgPool;

use super::common::setup_test_app;

// ============================================================================
// Logout Tests
// ============================================================================

/// Test successful logout with valid session
///
/// Expected: 204 No Content with Set-Cookie header to clear the cookie
#[sqlx::test]
async fn test_logout_success(pool: PgPool) {
    let (app, pool) = setup_test_app(pool).await;
    let server = TestServer::new(app);

    // Step 1: Register and verify user
    let register_payload = json!({
        "email": "logout-test@example.com",
        "password": "LogoutPass123",
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
        "logout-test@example.com"
    )
    .execute(&pool)
    .await
    .ok();

    // Step 2: Login to get a session cookie
    let login_payload = json!({
        "email": "logout-test@example.com",
        "password": "LogoutPass123"
    });

    let login_response = server.post("/auth/login").json(&login_payload).await;
    login_response.assert_status(axum::http::StatusCode::OK);

    // Extract the Set-Cookie header from login response
    let cookie_header = login_response.headers().get("set-cookie");
    assert!(
        cookie_header.is_some(),
        "Login response should have Set-Cookie header"
    );

    let cookie_str = cookie_header.unwrap().to_str().unwrap();
    assert!(cookie_str.contains("libiamo_session"));

    // Extract session_id from cookie (format: libiamo_session=<uuid>; HttpOnly...)
    let session_id = cookie_str
        .split(';')
        .next()
        .and_then(|s| s.strip_prefix("libiamo_session="))
        .expect("Unable to extract session ID from cookie");

    // Verify the session exists in the database
    let session_check = sqlx::query!("SELECT id FROM auth_sessions WHERE id::TEXT = $1", session_id)
        .fetch_optional(&pool)
        .await
        .expect("Database query failed");
    assert!(
        session_check.is_some(),
        "Session should exist in database after login"
    );

    // Step 3: Logout with the session cookie
    let logout_response = server
        .post("/auth/logout")
        .add_header("Cookie", format!("libiamo_session={}", session_id))
        .await;

    logout_response.assert_status(axum::http::StatusCode::NO_CONTENT);

    // Verify Set-Cookie header is present to clear the cookie
    let clear_cookie_header = logout_response.headers().get("set-cookie");
    assert!(
        clear_cookie_header.is_some(),
        "Logout response should have Set-Cookie header to clear cookie"
    );

    let clear_cookie_str = clear_cookie_header.unwrap().to_str().unwrap();
    assert!(
        clear_cookie_str.contains("Max-Age=0"),
        "Clear cookie should have Max-Age=0"
    );
    assert!(
        clear_cookie_str.contains("libiamo_session="),
        "Clear cookie should target libiamo_session"
    );

    // Verify the session is deleted from the database
    let session_deleted = sqlx::query!("SELECT id FROM auth_sessions WHERE id::TEXT = $1", session_id)
        .fetch_optional(&pool)
        .await
        .expect("Database query failed");
    assert!(
        session_deleted.is_none(),
        "Session should be deleted from database after logout"
    );
}

/// Test logout without session (no cookie)
///
/// Expected: 204 No Content (idempotent - logout is successful even if not logged in)
#[sqlx::test]
async fn test_logout_without_session(pool: PgPool) {
    let (app, _pool) = setup_test_app(pool).await;
    let server = TestServer::new(app);

    // Logout without any session cookie
    let logout_response = server.post("/auth/logout").await;

    // Should still return 204 No Content (idempotent operation)
    logout_response.assert_status(axum::http::StatusCode::NO_CONTENT);

    // Verify Set-Cookie header is present
    let clear_cookie_header = logout_response.headers().get("set-cookie");
    assert!(
        clear_cookie_header.is_some(),
        "Logout response should have Set-Cookie header even without session"
    );
}

/// Test logout with invalid session ID
///
/// Expected: 204 No Content (idempotent - still clears the cookie)
#[sqlx::test]
async fn test_logout_with_invalid_session(pool: PgPool) {
    let (app, _pool) = setup_test_app(pool).await;
    let server = TestServer::new(app);

    // Logout with fake session ID
    let logout_response = server
        .post("/auth/logout")
        .add_header("Cookie", "libiamo_session=00000000-0000-0000-0000-000000000000")
        .await;

    // Should still return 204 No Content (idempotent operation)
    logout_response.assert_status(axum::http::StatusCode::NO_CONTENT);

    // Verify Set-Cookie header is present to clear the cookie
    let clear_cookie_header = logout_response.headers().get("set-cookie");
    assert!(
        clear_cookie_header.is_some(),
        "Logout response should have Set-Cookie header"
    );
}
