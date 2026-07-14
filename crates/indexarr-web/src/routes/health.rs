use std::sync::Arc;

use axum::extract::State;
use axum::response::Json;

use crate::state::AppState;

pub async fn health(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let ready = state.is_ready();
    Json(serde_json::json!({
        "status": if ready { "ok" } else { "starting" },
        "version": env!("CARGO_PKG_VERSION"),
        "ready": ready,
    }))
}
