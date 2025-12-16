//! HTTP request handlers

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::state::AppState;

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

pub async fn create_message(
    State(_state): State<Arc<AppState>>,
    Json(request): Json<CreateMessageRequest>,
) -> Result<Json<MessageResponse>, StatusCode> {
    // TODO: Implement message creation logic
    let message_id = Uuid::new_v4();

    Ok(Json(MessageResponse {
        id: message_id,
        status: "created".to_string(),
    }))
}

pub async fn get_message(
    State(_state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<MessageResponse>, StatusCode> {
    // TODO: Implement message retrieval logic
    Ok(Json(MessageResponse {
        id,
        status: "found".to_string(),
    }))
}
