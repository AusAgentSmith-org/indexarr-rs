use std::sync::Arc;

use axum::Router;
use axum::extract::{Query, State};
use axum::response::Json;
use axum::routing::{get, post};
use sqlx::Row;

use crate::state::AppState;

async fn system_status(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, String)> {
    let pool = &state.pool;

    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM torrents")
        .fetch_one(pool)
        .await
        .map_err(db_err)?;
    let resolved: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM torrents WHERE resolved_at IS NOT NULL")
            .fetch_one(pool)
            .await
            .map_err(db_err)?;

    // DB version
    let db_version: String = sqlx::query_scalar("SELECT version()")
        .fetch_one(pool)
        .await
        .map_err(db_err)?;

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
        "uptime_seconds": state.started_at.elapsed().as_secs_f64(),
        "data_dir": state.settings.data_dir.display().to_string(),
    })))
}

async fn get_api_key(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let key = &state.settings.torznab_api_key;
    let masked = if key.len() > 4 {
        format!("{}...{}", &key[..2], &key[key.len() - 4..])
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

async fn generate_api_key(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
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

// --- Sync Preferences ---

static ALL_CATEGORIES: &[&str] = &[
    "movie",
    "tv_show",
    "music",
    "ebook",
    "comic",
    "audiobook",
    "game",
    "software",
    "xxx",
    "unknown",
];

#[derive(Debug, serde::Deserialize)]
struct SyncPrefsRequest {
    import_categories: Vec<String>,
    sync_comments: bool,
}

async fn get_sync_preferences(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    // Load from data dir or use defaults (all categories)
    let prefs_path = state.settings.data_dir.join("sync_preferences.json");
    let (import_categories, sync_comments) = if let Ok(data) = std::fs::read_to_string(&prefs_path)
    {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&data) {
            let cats: Vec<String> = v
                .get("import_categories")
                .and_then(|v| v.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_else(|| ALL_CATEGORIES.iter().map(|s| s.to_string()).collect());
            let comments = v
                .get("sync_comments")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            (cats, comments)
        } else {
            (ALL_CATEGORIES.iter().map(|s| s.to_string()).collect(), true)
        }
    } else {
        (ALL_CATEGORIES.iter().map(|s| s.to_string()).collect(), true)
    };

    Json(serde_json::json!({
        "all_categories": ALL_CATEGORIES,
        "import_categories": import_categories,
        "sync_comments": sync_comments,
    }))
}

async fn set_sync_preferences(
    State(state): State<Arc<AppState>>,
    Json(body): Json<SyncPrefsRequest>,
) -> Json<serde_json::Value> {
    let prefs = serde_json::json!({
        "import_categories": body.import_categories,
        "sync_comments": body.sync_comments,
    });

    let prefs_path = state.settings.data_dir.join("sync_preferences.json");
    let _ = std::fs::create_dir_all(&state.settings.data_dir);
    let _ = std::fs::write(
        &prefs_path,
        serde_json::to_string_pretty(&prefs).unwrap_or_default(),
    );

    Json(serde_json::json!({
        "all_categories": ALL_CATEGORIES,
        "import_categories": body.import_categories,
        "sync_comments": body.sync_comments,
    }))
}

// --- Logs ---

#[derive(Debug, serde::Deserialize)]
struct LogParams {
    limit: Option<usize>,
    category: Option<String>,
}

async fn get_recent_logs(
    State(state): State<Arc<AppState>>,
    Query(params): Query<LogParams>,
) -> Json<serde_json::Value> {
    let limit = params.limit.unwrap_or(500).min(5000);
    let entries = state
        .log_capture
        .get_recent(limit, params.category.as_deref());
    Json(serde_json::json!({
        "entries": entries,
        "categories": state.log_capture.categories(),
        "debug_enabled": state.log_capture.debug_enabled(),
    }))
}

async fn get_log_categories(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    Json(serde_json::json!(state.log_capture.categories()))
}

async fn toggle_debug(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let new_val = !state.log_capture.debug_enabled();
    state.log_capture.set_debug_enabled(new_val);
    Json(serde_json::json!({ "debug_enabled": new_val }))
}

async fn logs_ws(
    State(state): State<Arc<AppState>>,
    ws: axum::extract::WebSocketUpgrade,
) -> impl axum::response::IntoResponse {
    ws.on_upgrade(move |mut socket| async move {
        use axum::extract::ws::Message;
        let mut rx = state.log_capture.subscribe();

        loop {
            tokio::select! {
                result = rx.recv() => {
                    match result {
                        Ok(entry) => {
                            let json = serde_json::to_string(&entry).unwrap_or_default();
                            if socket.send(Message::Text(json.into())).await.is_err() {
                                break;
                            }
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                        Err(_) => break,
                    }
                }
                msg = socket.recv() => {
                    match msg {
                        Some(Ok(_)) => continue,
                        _ => break,
                    }
                }
            }
        }
    })
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
        .route(
            "/system/sync/preferences",
            get(get_sync_preferences).post(set_sync_preferences),
        )
        .route("/system/logs/recent", get(get_recent_logs))
        .route("/system/logs/categories", get(get_log_categories))
        .route("/system/logs/debug", post(toggle_debug))
        .route("/system/logs/ws", get(logs_ws))
}
