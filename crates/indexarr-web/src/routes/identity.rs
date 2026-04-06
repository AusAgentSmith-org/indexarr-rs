use std::sync::Arc;

use axum::Router;
use axum::extract::State;
use axum::response::Json;
use axum::routing::{get, post};
use serde::Deserialize;

use crate::state::AppState;

async fn identity_status(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let identity = state.identity.read();
    let needs_onboarding = identity.needs_onboarding();
    let recovery_key = if needs_onboarding {
        identity.pending_recovery_key()
    } else {
        None
    };

    Json(serde_json::json!({
        "initialized": identity.is_initialized(),
        "needs_onboarding": needs_onboarding,
        "contributor_id": identity.contributor_id(),
        "public_key": identity.public_key_b64(),
        "recovery_key": recovery_key,
    }))
}

#[derive(Debug, Deserialize)]
struct RestoreRequest {
    recovery_key: String,
}

async fn restore_identity(
    State(state): State<Arc<AppState>>,
    Json(body): Json<RestoreRequest>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, String)> {
    let mut identity = state.identity.write();
    identity
        .restore_from_recovery_key(&body.recovery_key)
        .map_err(|e| (axum::http::StatusCode::BAD_REQUEST, e.to_string()))?;
    identity.acknowledge_onboarding();

    Ok(Json(serde_json::json!({
        "status": "restored",
        "contributor_id": identity.contributor_id(),
    })))
}

async fn acknowledge_onboarding(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let identity = state.identity.read();
    identity.acknowledge_onboarding();
    Json(serde_json::json!({ "status": "acknowledged" }))
}

async fn get_bans(
    State(_state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    // Ban list managed separately; return empty for now
    Json(serde_json::json!({ "bans": {}, "total": 0 }))
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/identity/status", get(identity_status))
        .route("/identity/restore", post(restore_identity))
        .route("/identity/acknowledge", post(acknowledge_onboarding))
        .route("/identity/bans", get(get_bans))
}
