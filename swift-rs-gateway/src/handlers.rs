use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::state::AppState;
use crate::system::{ChainStatus, ProposalView, PublicRecord, SwapReceipt, SystemError, TransferReceipt};

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateMessageRequest {
    pub message_type: String,
    pub sender: String,
    pub receiver: String,
    pub payload: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MessageResponse {
    pub id: Uuid,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
}

#[derive(Debug, Deserialize)]
pub struct TransferRequest {
    pub from: String,
    pub to: String,
    pub asset: String,
    pub amount: u128,
    pub purpose: String,
}

#[derive(Debug, Deserialize)]
pub struct SwapRequest {
    pub from: String,
    pub to: String,
    pub source: String,
    pub dest: String,
    pub amount: u128,
    pub purpose: String,
}

#[derive(Debug, Deserialize)]
pub struct ProposeRequest {
    pub proposer: String,
    pub title: String,
    pub payload: String,
}

#[derive(Debug, Deserialize)]
pub struct VoteRequest {
    pub voter: String,
    pub proposal_id: u64,
    pub support: bool,
}

#[derive(Debug, Serialize)]
pub struct ErrorBody {
    pub error: String,
}

#[derive(Debug, Serialize)]
pub struct BalanceResponse {
    pub alias: String,
    pub asset: String,
    pub amount: u128,
}

#[derive(Debug, Serialize)]
pub struct ProposeResponse {
    pub id: u64,
}

#[derive(Debug, Serialize)]
pub struct TallyResponse {
    pub status: String,
}

pub async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy".into(),
        version: env!("CARGO_PKG_VERSION").into(),
    })
}

pub async fn create_message(
    State(_state): State<Arc<AppState>>,
    Json(_request): Json<CreateMessageRequest>,
) -> Result<Json<MessageResponse>, StatusCode> {
    Ok(Json(MessageResponse {
        id: Uuid::new_v4(),
        status: "created".into(),
    }))
}

pub async fn get_message(
    State(_state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<MessageResponse>, StatusCode> {
    Ok(Json(MessageResponse {
        id,
        status: "found".into(),
    }))
}

pub async fn metrics(State(state): State<Arc<AppState>>) -> Result<String, StatusCode> {
    let system = state.system.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(system.metrics())
}

pub async fn status(State(state): State<Arc<AppState>>) -> Result<Json<ChainStatus>, StatusCode> {
    let system = state.system.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(system.status()))
}

pub async fn public_explorer(State(state): State<Arc<AppState>>) -> Result<Json<Vec<PublicRecord>>, StatusCode> {
    let system = state.system.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(system.public_records()))
}

pub async fn open_regulator(
    State(state): State<Arc<AppState>>,
    Path(tx_id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    let secret = headers
        .get("x-regulator-key")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");
    let system = state.system.lock().map_err(|_| bad("lock"))?;
    let revealed = system.open_for_regulator(&tx_id, secret).map_err(bad_err)?;
    Ok(Json(serde_json::json!({
        "from": revealed.from.to_hex(),
        "to": revealed.to.to_hex(),
        "asset": revealed.asset,
        "amount": revealed.amount.to_string(),
        "purpose": revealed.purpose,
    })))
}

pub async fn transfer(
    State(state): State<Arc<AppState>>,
    Json(request): Json<TransferRequest>,
) -> Result<Json<TransferReceipt>, (StatusCode, Json<ErrorBody>)> {
    let mut system = state.system.lock().map_err(|_| bad("lock"))?;
    system
        .transfer(&request.from, &request.to, &request.asset, request.amount, &request.purpose)
        .map(Json)
        .map_err(bad_err)
}

pub async fn swap(
    State(state): State<Arc<AppState>>,
    Json(request): Json<SwapRequest>,
) -> Result<Json<SwapReceipt>, (StatusCode, Json<ErrorBody>)> {
    let mut system = state.system.lock().map_err(|_| bad("lock"))?;
    system
        .swap(
            &request.from,
            &request.to,
            &request.source,
            &request.dest,
            request.amount,
            &request.purpose,
        )
        .map(Json)
        .map_err(bad_err)
}

pub async fn balance(
    State(state): State<Arc<AppState>>,
    Path((alias, asset)): Path<(String, String)>,
) -> Result<Json<BalanceResponse>, (StatusCode, Json<ErrorBody>)> {
    let system = state.system.lock().map_err(|_| bad("lock"))?;
    let amount = system.balance(&alias, &asset).map_err(bad_err)?;
    Ok(Json(BalanceResponse { alias, asset, amount }))
}

pub async fn proposals(State(state): State<Arc<AppState>>) -> Result<Json<Vec<ProposalView>>, StatusCode> {
    let system = state.system.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(system.proposals()))
}

pub async fn propose(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ProposeRequest>,
) -> Result<Json<ProposeResponse>, (StatusCode, Json<ErrorBody>)> {
    let mut system = state.system.lock().map_err(|_| bad("lock"))?;
    let id = system
        .propose(&request.proposer, &request.title, &request.payload)
        .map_err(bad_err)?;
    Ok(Json(ProposeResponse { id }))
}

pub async fn vote(
    State(state): State<Arc<AppState>>,
    Json(request): Json<VoteRequest>,
) -> Result<StatusCode, (StatusCode, Json<ErrorBody>)> {
    let mut system = state.system.lock().map_err(|_| bad("lock"))?;
    system
        .vote(&request.voter, request.proposal_id, request.support)
        .map_err(bad_err)?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn tally(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u64>,
) -> Result<Json<TallyResponse>, (StatusCode, Json<ErrorBody>)> {
    let mut system = state.system.lock().map_err(|_| bad("lock"))?;
    let status = system.tally(id).map_err(bad_err)?;
    Ok(Json(TallyResponse { status }))
}

pub async fn export_rail(
    State(state): State<Arc<AppState>>,
    Path((rail, tx_id)): Path<(String, String)>,
) -> Result<String, (StatusCode, Json<ErrorBody>)> {
    let system = state.system.lock().map_err(|_| bad("lock"))?;
    system.export_message(&rail, &tx_id).map_err(bad_err)
}

fn bad(error: &str) -> (StatusCode, Json<ErrorBody>) {
    (StatusCode::BAD_REQUEST, Json(ErrorBody { error: error.into() }))
}

fn bad_err(error: SystemError) -> (StatusCode, Json<ErrorBody>) {
    (StatusCode::BAD_REQUEST, Json(ErrorBody { error: error.to_string() }))
}
