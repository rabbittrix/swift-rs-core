//! Swift-RS Gateway: API Gateway for gRPC and REST
//!
//! Provides unified API access to the Swift-RS core engine with support for
//! both REST and gRPC protocols, implementing the Strangler Fig pattern.

use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;
use tracing_subscriber::EnvFilter;

mod handlers;
mod state;

use state::AppState;

#[derive(Debug, Serialize, Deserialize)]
struct HealthResponse {
    status: String,
    version: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(LevelFilter::INFO.into()))
        .init();

    info!("Starting Swift-RS Gateway...");

    let app_state = Arc::new(AppState::new());

    // Build REST API
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/api/v1/messages", post(handlers::create_message))
        .route("/api/v1/messages/:id", get(handlers::get_message))
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
    info!("Swift-RS Gateway listening on http://0.0.0.0:8080");

    axum::serve(listener, app).await?;

    Ok(())
}

async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}
