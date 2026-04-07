use std::sync::Arc;

use axum::Router;
use axum::extract::{Query, State};
use axum::response::Json;
use axum::routing::get;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;

use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct QueueParams {
    #[serde(default = "default_limit")]
    limit: i64,
    #[serde(default)]
    offset: i64,
}

fn default_limit() -> i64 {
    50
}

#[derive(Debug, Serialize)]
pub struct QueueItem {
    info_hash: String,
    source: String,
    discovered_at: Option<String>,
    resolve_attempts: i32,
    observations: i32,
}

#[derive(Debug, Serialize)]
pub struct QueueResponse {
    results: Vec<QueueItem>,
    total: i64,
    offset: i64,
    limit: i64,
}

async fn get_queue(
    State(state): State<Arc<AppState>>,
    Query(params): Query<QueueParams>,
) -> Result<Json<QueueResponse>, (axum::http::StatusCode, String)> {
    let pool = &state.pool;
    let limit = params.limit.clamp(1, 100);
    let offset = params.offset.max(0);

    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM torrents WHERE resolved_at IS NULL")
        .fetch_one(pool)
        .await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let rows = sqlx::query(
        "SELECT info_hash, source, discovered_at, resolve_attempts, observations \
         FROM torrents WHERE resolved_at IS NULL \
         ORDER BY discovered_at DESC LIMIT $1 OFFSET $2",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let items: Vec<QueueItem> = rows
        .iter()
        .map(|r| QueueItem {
            info_hash: r.get("info_hash"),
            source: r.get("source"),
            discovered_at: r
                .get::<Option<DateTime<Utc>>, _>("discovered_at")
                .map(|d| d.to_rfc3339()),
            resolve_attempts: r.get("resolve_attempts"),
            observations: r.get("observations"),
        })
        .collect();

    Ok(Json(QueueResponse {
        results: items,
        total,
        offset,
        limit,
    }))
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/queue", get(get_queue))
}
