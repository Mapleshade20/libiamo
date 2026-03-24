use std::env;

use axum::{Router, routing::post};
use dotenvy::dotenv;
use sqlx::postgres::PgPoolOptions;
use tokio::net::TcpListener;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use libiamo::handlers;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables from .env
    dotenv().ok();

    // Initialize tracing for logging
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    // Connect to the database
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set in .env");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    // Build the application with routes
    let app = Router::new()
        .route("/auth/register", post(handlers::auth::register))
        .with_state(pool);

    // Start the server
    let addr = env::var("ADDRESS").expect("Env variable `ADDRESS` should be set");
    tracing::info!("Listening on {}", addr);

    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
