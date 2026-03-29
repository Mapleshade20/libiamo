//! Common utilities and setup for authentication tests

use axum::{Router, routing::post};
use sqlx::PgPool;

use libiamo::handlers::auth::{login, register, verify_email, logout};

/// Sets up a test app with all auth routes and returns both the router and pool
/// 
/// This setup includes:
/// - POST /auth/register
/// - POST /auth/verify-email
/// - POST /auth/login
/// - POST /auth/logout
pub async fn setup_test_app(pool: PgPool) -> (Router, PgPool) {
    let pool_clone = pool.clone();
    let router = Router::new()
        .route("/auth/register", post(register))
        .route("/auth/verify-email", post(verify_email))
        .route("/auth/login", post(login))
        .route("/auth/logout", post(logout))
        .with_state(pool_clone);

    (router, pool)
}
