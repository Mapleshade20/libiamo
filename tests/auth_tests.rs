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
        "password": "Secure123",
        "target_languages": ["en"],
        "native_language": "zh",
        "timezone": "Europe/Rome"
    });

    let response = server.post("/auth/register").json(&payload).await;

    response.assert_status_success();
    let body: serde_json::Value = response.json();
    assert_eq!(body.get("email").unwrap(), "test@example.com");
    assert_eq!(body.get("target_languages").unwrap(), &json!(["en"]));
    assert_eq!(body.get("native_language").unwrap(), "zh");
    assert!(body.get("created_at").is_some());
}

#[sqlx::test]
async fn test_register_invalid_email(pool: PgPool) {
    let app = setup_test_app(pool).await;
    let server = TestServer::new(app);

    let payload = json!({
        "email": "invalid-email",
        "password": "Password123",
        "target_languages": ["en"],
        "native_language": "en"
    });

    let response = server.post("/auth/register").json(&payload).await;

    response.assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
}

#[sqlx::test]
async fn test_register_unverified_retry(pool: PgPool) {
    let app = setup_test_app(pool.clone()).await;
    let server = TestServer::new(app);

    let payload = json!({
        "email": "retry@example.com",
        "password": "Password123",
        "target_languages": ["en", "es"],
        "native_language": "en"
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

    // Verify user_learning_profiles were created for both languages
    let profiles = sqlx::query!(
        r#"
        SELECT language::text as "language!"
        FROM user_learning_profiles
        WHERE user_id = (SELECT id FROM users WHERE email = $1)
        ORDER BY language
        "#,
        "retry@example.com"
    )
    .fetch_all(&pool)
    .await
    .unwrap();

    assert_eq!(profiles.len(), 2);
    assert_eq!(profiles[0].language, "en");
    assert_eq!(profiles[1].language, "es");

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

#[sqlx::test]
async fn test_register_empty_target_languages(pool: PgPool) {
    let app = setup_test_app(pool).await;
    let server = TestServer::new(app);

    let payload = json!({
        "email": "empty-lang@example.com",
        "password": "Password123",
        "target_languages": [],
        "native_language": "en"
    });

    let response = server.post("/auth/register").json(&payload).await;

    // This should fail because target_languages must not be empty (validation or handler logic)
    response.assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
}

#[sqlx::test]
async fn test_register_unsupported_language(pool: PgPool) {
    let app = setup_test_app(pool).await;
    let server = TestServer::new(app);

    let payload = json!({
        "email": "unsupported@example.com",
        "password": "Password123",
        "target_languages": ["jp"], // Not in En, Es, Fr
        "native_language": "en"
    });

    let response = server.post("/auth/register").json(&payload).await;

    response.assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
}
