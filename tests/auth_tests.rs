use axum::{routing::post, Router};
use axum_test::TestServer;
use libiamo::handlers::auth::register;
use libiamo::models::auth::RegisterRequest;
use serde_json::json;
use sqlx::PgPool;

async fn setup_test_app(pool: PgPool) -> Router {
    Router::new()
        .route("/auth/register", post(register))
        .with_state(pool)
}

#[sqlx::test]
async fn test_register_success(pool: PgPool) {
    let app = setup_test_app(pool).await;
    let server = TestServer::new(app).unwrap();

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
async fn test_register_duplicate_email(pool: PgPool) {
    let app = setup_test_app(pool).await;
    let server = TestServer::new(app).unwrap();

    let payload = json!({
        "email": "dup@example.com",
        "password": "password123",
        "nickname": "dup",
        "role": "learner",
        "target_language": "en",
        "native_language": "en",
        "level_self_assign": 1
    });

    // First registration
    server.post("/auth/register").json(&payload).await.assert_status_success();

    // Duplicate registration
    let response = server.post("/auth/register").json(&payload).await;
    
    // Should fail with 409 Conflict (as handled by our AppError)
    response.assert_status(axum::http::StatusCode::CONFLICT);
}

#[sqlx::test]
async fn test_register_invalid_email(pool: PgPool) {
    let app = setup_test_app(pool).await;
    let server = TestServer::new(app).unwrap();

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
