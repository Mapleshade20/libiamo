use axum::{Router, routing::post};
use axum_test::TestServer;
use serde_json::json;
use sqlx::PgPool;

use libiamo::handlers::auth::register;

async fn setup_test_app(pool: PgPool) -> Router {
    Router::new()
        .route("/auth/register", post(register))
        .with_state(pool)
}

#[sqlx::test]
async fn test_register_success(pool: PgPool) {
    let app = setup_test_app(pool).await;
    let server = TestServer::new(app);

    let payload = json!({
        "email": "test@example.com",
        "password": "secure_password123",
        "nickname": "tester",
        "role": "learner",
        "target_language": "en",
        "native_language": "zh",
        "timezone": "Europe/Rome",
        "level_self_assign": 1
    });

    let response = server.post("/auth/register").json(&payload).await;

    response.assert_status_success();
    let body: serde_json::Value = response.json();
    assert!(body.get("user_id").is_some());
    assert_eq!(body.get("message").unwrap(), "User registered successfully");
}

#[sqlx::test]
async fn test_register_invalid_email(pool: PgPool) {
    let app = setup_test_app(pool).await;
    let server = TestServer::new(app);

    let payload = json!({
        "email": "invalid-email", // Invalid email format
        "password": "password123",
        "nickname": "tester",
        "role": "learner",
        "target_language": "en",
        "native_language": "en",
        "level_self_assign": 1
    });

    let response = server.post("/auth/register").json(&payload).await;

    // Should fail with 400 Bad Request (Validation error)
    response.assert_status(axum::http::StatusCode::BAD_REQUEST);
}

#[sqlx::test]
async fn test_register_unverified_retry(pool: PgPool) {
    let app = setup_test_app(pool.clone()).await;
    let server = TestServer::new(app);

    let payload = json!({
        "email": "retry@example.com",
        "password": "password123",
        "nickname": "retry",
        "role": "learner",
        "target_language": "en",
        "native_language": "en",
        "level_self_assign": 1
    });

    // First registration
    server
        .post("/auth/register")
        .json(&payload)
        .await
        .assert_status_success();

    // Try again immediately - should fail with 429 Too Many Requests
    let response = server.post("/auth/register").json(&payload).await;
    response.assert_status(axum::http::StatusCode::TOO_MANY_REQUESTS);

    // Manually set created_at back in time
    sqlx::query!(
        "UPDATE users SET created_at = now() - interval '21 minutes' WHERE email = $1",
        "retry@example.com"
    )
    .execute(&pool)
    .await
    .unwrap();

    // Try again - should succeed
    let response = server.post("/auth/register").json(&payload).await;
    response.assert_status_success();

    // Manually verify the user
    sqlx::query!(
        "UPDATE users SET is_verified = true WHERE email = $1",
        "retry@example.com"
    )
    .execute(&pool)
    .await
    .unwrap();

    // Duplicate registration again
    let response = server.post("/auth/register").json(&payload).await;

    // Now it should fail with 409 Conflict
    response.assert_status(axum::http::StatusCode::CONFLICT);
}
