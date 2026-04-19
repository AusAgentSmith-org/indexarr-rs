use std::sync::Arc;

use axum::Router;
use axum::extract::State;
use axum::response::Json;
use axum::routing::{get, post};
use serde::Deserialize;

use crate::state::AppState;

async fn identity_status(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
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

async fn acknowledge_onboarding(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let identity = state.identity.read();
    identity.acknowledge_onboarding();
    Json(serde_json::json!({ "status": "acknowledged" }))
}

async fn get_bans(State(_state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    Json(serde_json::json!({ "bans": {}, "total": 0 }))
}

async fn get_epoch_info(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let epoch = indexarr_sync::epoch::get_current_epoch(&state.settings.data_dir);
    let decl = indexarr_sync::epoch::get_declaration(&state.settings.data_dir);

    Json(serde_json::json!({
        "epoch": epoch,
        "reason": decl.as_ref().map(|d| d.reason.as_str()),
        "effective_at": decl.as_ref().map(|d| d.effective_at.to_rfc3339()),
        "seed_contributors": decl.as_ref().map(|d| &d.seed_contributors).unwrap_or(&vec![]),
        "seed_only_hours": decl.as_ref().map(|d| d.seed_only_hours).unwrap_or(0.0),
        "seed_only_active": indexarr_sync::epoch::in_seed_only_mode(&state.settings.data_dir),
        "signature_valid": decl.as_ref().map(|d| {
            indexarr_sync::epoch::verify_declaration(d, &state.settings.swarm_maintainer_pubkey)
        }).unwrap_or(false),
        "maintainer_pubkey_set": !state.settings.swarm_maintainer_pubkey.is_empty(),
    }))
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/identity/status", get(identity_status))
        .route("/identity/restore", post(restore_identity))
        .route("/identity/acknowledge", post(acknowledge_onboarding))
        .route("/identity/bans", get(get_bans))
        .route("/identity/epoch", get(get_epoch_info))
}
