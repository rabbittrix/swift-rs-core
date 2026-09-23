//! REST gateway for screened CBDC settlement.

pub mod handlers;
pub mod state;
pub mod system;

use std::sync::Arc;

use axum::{routing::get, routing::post, Router};
use tracing::info;
use tracing_subscriber::{filter::LevelFilter, EnvFilter};

use crate::handlers::{
    balance, create_message, export_rail, get_message, health_check, metrics, open_regulator,
    proposals, public_explorer, status, swap, tally, transfer, vote,
};
use crate::state::AppState;

pub async fn serve() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(LevelFilter::INFO.into()))
        .init();

    info!("Starting Swift-RS Gateway...");
    let app_state = Arc::new(AppState::new()?);

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/metrics", get(metrics))
        .route("/api/v1/messages", post(create_message))
        .route("/api/v1/messages/{id}", get(get_message))
        .route("/api/v1/chain/status", get(status))
        .route("/api/v1/explorer", get(public_explorer))
        .route("/api/v1/explorer/{tx_id}", get(open_regulator))
        .route("/api/v1/transfers", post(transfer))
        .route("/api/v1/swaps", post(swap))
        .route("/api/v1/balances/{alias}/{asset}", get(balance))
        .route("/api/v1/governance/proposals", get(proposals).post(handlers::propose))
        .route("/api/v1/governance/votes", post(vote))
        .route("/api/v1/governance/proposals/{id}/tally", post(tally))
        .route("/api/v1/rails/{rail}/{tx_id}", get(export_rail))
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
    info!("Swift-RS Gateway listening on http://0.0.0.0:8080");
    axum::serve(listener, app).await?;
    Ok(())
}
