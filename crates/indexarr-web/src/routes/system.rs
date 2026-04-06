use std::sync::Arc;

use axum::Router;
use axum::extract::State;
use axum::response::Json;
use axum::routing::{get, post};
use sqlx::Row;

use crate::state::AppState;

async fn system_status(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, String)> {
    let pool = &state.pool;

    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM torrents")
        .fetch_one(pool).await.map_err(db_err)?;
    let resolved: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM torrents WHERE resolved_at IS NOT NULL")
        .fetch_one(pool).await.map_err(db_err)?;

    // DB version
    let db_version: String = sqlx::query_scalar("SELECT version()")
        .fetch_one(pool).await.map_err(db_err)?;

    Ok(Json(serde_json::json!({
        "version": "0.1.0",
        "runtime": "rust",
        "is_docker": std::path::Path::new("/.dockerenv").exists(),
        "db_backend": state.settings.db_backend.to_string(),
        "db_version": db_version,
        "host": state.settings.host,
        "port": state.settings.port,
        "workers": state.settings.workers,
        "debug": state.settings.debug,
        "total_hashes": total,
        "resolved_hashes": resolved,
    })))
}

async fn get_api_key(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let key = &state.settings.torznab_api_key;
    let masked = if key.len() > 4 {
        format!("{}...{}", &key[..2], &key[key.len()-4..])
    } else if key.is_empty() {
        "(not set)".to_string()
    } else {
        "****".to_string()
    };

    Json(serde_json::json!({
        "api_key": masked,
        "configured": !key.is_empty(),
    }))
}

async fn generate_api_key(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    // Generate a random 32-char hex key
    let key: String = (0..32)
        .map(|_| format!("{:x}", rand::random::<u8>()))
        .collect::<Vec<_>>()
        .join("")
        .chars()
        .take(32)
        .collect();

    // Write to data dir
    let key_path = state.settings.data_dir.join("apikey");
    let _ = std::fs::create_dir_all(&state.settings.data_dir);
    let _ = std::fs::write(&key_path, &key);

    Json(serde_json::json!({
        "api_key": key,
        "message": "API key generated. Set INDEXARR_TORZNAB_API_KEY to persist across restarts.",
    }))
}

fn db_err(e: sqlx::Error) -> (axum::http::StatusCode, String) {
    tracing::error!(error = %e, "database error");
    (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/system/status", get(system_status))
        .route("/system/apikey", get(get_api_key))
        .route("/system/apikey/generate", post(generate_api_key))
}
