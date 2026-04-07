use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Json, Response};
use axum::routing::{get, post};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use sqlx::Row;

use crate::state::AppState;

async fn get_sync_dashboard(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let pool = &state.pool;

    // Peer list from DB
    let peer_rows = sqlx::query(
        "SELECT peer_id, peer_url, last_sync_at, last_sequence, fail_count, reputation, source, first_seen \
         FROM sync_state ORDER BY reputation DESC"
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let peers: Vec<serde_json::Value> = peer_rows.iter().map(|r| {
        let reputation: f64 = r.get("reputation");
        let fail_count: i32 = r.get("fail_count");
        serde_json::json!({
            "peer_id": r.get::<String, _>("peer_id"),
            "url": r.get::<Option<String>, _>("peer_url"),
            "source": r.get::<String, _>("source"),
            "last_sequence": r.get::<i64, _>("last_sequence"),
            "fail_count": fail_count,
            "reputation": reputation,
            "healthy": reputation > 20.0,
            "last_seen": r.get::<Option<DateTime<Utc>>, _>("last_sync_at").map(|d| d.to_rfc3339()),
            "first_seen": r.get::<Option<DateTime<Utc>>, _>("first_seen").map(|d| d.to_rfc3339()),
        })
    }).collect();

    let healthy_count = peers
        .iter()
        .filter(|p| p.get("healthy").and_then(|v| v.as_bool()).unwrap_or(false))
        .count();

    // Sequence from file
    let seq_path = state.settings.data_dir.join("sync").join("sequence");
    let sequence: i64 = std::fs::read_to_string(&seq_path)
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0);

    // Total imported (source = 'sync')
    let total_imported: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM torrents WHERE source = 'sync'")
            .fetch_one(pool)
            .await
            .unwrap_or(0);

    // Total exported
    let total_exported: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM torrents WHERE sync_sequence IS NOT NULL")
            .fetch_one(pool)
            .await
            .unwrap_or(0);

    let epoch = indexarr_sync::epoch::get_current_epoch(&state.settings.data_dir);

    Json(serde_json::json!({
        "contributor_id": state.identity.read().contributor_id(),
        "enabled": state.settings.sync_enabled,
        "sequence": sequence,
        "total_imported": total_imported,
        "total_exported": total_exported,
        "gossip_rounds": 0,
        "last_gossip_at": null,
        "last_export_at": null,
        "last_discovery_at": null,
        "import_interval": state.settings.sync_import_interval,
        "export_interval": state.settings.sync_export_interval,
        "discovery_interval": state.settings.sync_discovery_interval,
        "gossip_fanout": state.settings.gossip_fanout,
        "max_peers": state.settings.gossip_max_peers,
        "epoch": epoch,
        "peers": peers,
        "channels": [
            {"name": "HTTP/PEX", "running": state.settings.sync_enabled},
            {"name": "DHT", "running": state.settings.sync_dht_enabled && state.settings.sync_enabled},
        ],
        "activity": [],
        "bootstrap": null,
    }))
}

async fn get_sync_latest(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let seq_path = state.settings.data_dir.join("sync").join("sequence");
    let sequence: i64 = std::fs::read_to_string(&seq_path)
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0);

    Json(serde_json::json!({
        "sequence": sequence,
        "contributor_id": state.identity.read().contributor_id(),
        "epoch": indexarr_sync::epoch::get_current_epoch(&state.settings.data_dir),
    }))
}

async fn get_sync_status(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let pool = &state.pool;
    let peer_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sync_state")
        .fetch_one(pool)
        .await
        .unwrap_or(0);

    Json(serde_json::json!({
        "enabled": state.settings.sync_enabled,
        "peer_count": peer_count,
        "epoch": indexarr_sync::epoch::get_current_epoch(&state.settings.data_dir),
    }))
}

async fn get_sync_manifest(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let identity = state.identity.read();
    let contributor_id = identity.contributor_id().unwrap_or("unknown").to_string();
    drop(identity);

    let epoch = indexarr_sync::epoch::get_current_epoch(&state.settings.data_dir);
    let exporter = indexarr_sync::delta::DeltaExporter::new(
        &state.settings.data_dir,
        state.settings.sync_max_delta_size,
    );
    let manifest = exporter.build_manifest(&contributor_id, epoch);

    Json(serde_json::to_value(&manifest).unwrap_or_default())
}

async fn get_sync_peers(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let pool = &state.pool;
    let rows = sqlx::query(
        "SELECT peer_id, peer_url, last_sequence FROM sync_state WHERE peer_url IS NOT NULL",
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let peers: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "contributor_id": r.get::<String, _>("peer_id"),
                "url": r.get::<Option<String>, _>("peer_url"),
                "sequence": r.get::<i64, _>("last_sequence"),
            })
        })
        .collect();

    Json(serde_json::json!({ "peers": peers }))
}

async fn get_sync_delta(
    State(state): State<Arc<AppState>>,
    Path(content_hash): Path<String>,
) -> Response {
    let sync_dir = state.settings.data_dir.join("sync");

    // Find delta file by content hash
    if let Ok(entries) = std::fs::read_dir(&sync_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("gz") {
                if let Ok(data) = std::fs::read(&path) {
                    use sha2::{Digest, Sha256};
                    let hash = hex::encode(Sha256::digest(&data));
                    if hash == content_hash {
                        return (
                            StatusCode::OK,
                            [(header::CONTENT_TYPE, "application/gzip")],
                            data,
                        )
                            .into_response();
                    }
                }
            }
        }
    }

    (StatusCode::NOT_FOUND, "delta not found").into_response()
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/sync/dashboard", get(get_sync_dashboard))
        .route("/sync/latest", get(get_sync_latest))
        .route("/sync/status", get(get_sync_status))
        .route("/sync/manifest", get(get_sync_manifest))
        .route("/sync/peers", get(get_sync_peers))
        .route("/sync/delta/{content_hash}", get(get_sync_delta))
}
