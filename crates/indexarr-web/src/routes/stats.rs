use std::sync::Arc;

use axum::Router;
use axum::extract::{Query, State};
use axum::response::Json;
use axum::routing::get;
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;

use crate::state::AppState;

// --- Stats ---

#[derive(Debug, Serialize)]
pub struct SourceCount {
    source: String,
    count: i64,
}

#[derive(Debug, Serialize)]
pub struct ContentTypeCount {
    content_type: String,
    count: i64,
}

#[derive(Debug, Serialize)]
pub struct StatsResponse {
    total: i64,
    resolved: i64,
    unresolved: i64,
    pending: i64,
    dead: i64,
    no_peers: i64,
    private: i64,
    with_nfo: i64,
    announced_count: i64,
    pending_announce_count: i64,
    by_source: Vec<SourceCount>,
    by_content_type: Vec<ContentTypeCount>,
}

async fn get_stats(
    State(state): State<Arc<AppState>>,
) -> Result<Json<StatsResponse>, (axum::http::StatusCode, String)> {
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
    let no_peers: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM torrents WHERE no_peers IS TRUE")
        .fetch_one(pool)
        .await
        .map_err(db_err)?;
    let dead: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM torrents WHERE resolved_at IS NULL AND no_peers IS TRUE",
    )
    .fetch_one(pool)
    .await
    .map_err(db_err)?;
    let private: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM torrents WHERE private IS TRUE")
        .fetch_one(pool)
        .await
        .map_err(db_err)?;
    let with_nfo: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM torrents WHERE nfo IS NOT NULL")
        .fetch_one(pool)
        .await
        .map_err(db_err)?;
    let announced_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM torrents WHERE announced_at IS NOT NULL")
            .fetch_one(pool)
            .await
            .map_err(db_err)?;
    let pending_announce_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM torrents WHERE announced_at IS NULL AND name IS NOT NULL AND no_peers IS NOT TRUE"
    ).fetch_one(pool).await.map_err(db_err)?;

    let src_rows = sqlx::query(
        "SELECT source, COUNT(*) as cnt FROM torrents GROUP BY source ORDER BY cnt DESC",
    )
    .fetch_all(pool)
    .await
    .map_err(db_err)?;
    let by_source: Vec<SourceCount> = src_rows
        .iter()
        .map(|r| SourceCount {
            source: r.get("source"),
            count: r.get("cnt"),
        })
        .collect();

    let ct_rows = sqlx::query("SELECT content_type, COUNT(*) as cnt FROM torrent_content WHERE content_type IS NOT NULL GROUP BY content_type ORDER BY cnt DESC")
        .fetch_all(pool).await.map_err(db_err)?;
    let by_content_type: Vec<ContentTypeCount> = ct_rows
        .iter()
        .map(|r| ContentTypeCount {
            content_type: r.get("content_type"),
            count: r.get("cnt"),
        })
        .collect();

    let unresolved = total - resolved;
    let pending = unresolved - dead;

    Ok(Json(StatsResponse {
        total,
        resolved,
        unresolved,
        pending,
        dead,
        no_peers,
        private,
        with_nfo,
        announced_count,
        pending_announce_count,
        by_source,
        by_content_type,
    }))
}

// --- Trending ---

#[derive(Debug, Deserialize)]
pub struct TrendingParams {
    #[serde(default = "default_hours")]
    hours: i64,
    #[serde(default = "default_trend_limit")]
    limit: i64,
}

fn default_hours() -> i64 {
    12
}
fn default_trend_limit() -> i64 {
    10
}

#[derive(Debug, Serialize)]
pub struct TrendingItem {
    info_hash: String,
    name: String,
    size: i64,
    content_type: Option<String>,
    seed_count: i32,
    peer_count: i32,
    resolved_at: Option<String>,
}

async fn get_trending(
    State(state): State<Arc<AppState>>,
    Query(params): Query<TrendingParams>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, String)> {
    let cutoff = Utc::now() - Duration::hours(params.hours.clamp(1, 168));
    let rows = sqlx::query(
        "SELECT t.info_hash, t.name, t.size, t.seed_count, t.peer_count, t.resolved_at, c.content_type \
         FROM torrents t LEFT JOIN torrent_content c ON t.info_hash = c.info_hash \
         WHERE t.resolved_at IS NOT NULL AND t.resolved_at >= $1 \
         ORDER BY t.seed_count DESC LIMIT $2"
    )
    .bind(cutoff)
    .bind(params.limit.clamp(1, 50))
    .fetch_all(&state.pool)
    .await
    .map_err(db_err)?;

    let items: Vec<TrendingItem> = rows
        .iter()
        .map(|r| TrendingItem {
            info_hash: r.get("info_hash"),
            name: r.get::<Option<String>, _>("name").unwrap_or_default(),
            size: r.get::<Option<i64>, _>("size").unwrap_or(0),
            content_type: r.get("content_type"),
            seed_count: r.get::<Option<i32>, _>("seed_count").unwrap_or(0),
            peer_count: r.get::<Option<i32>, _>("peer_count").unwrap_or(0),
            resolved_at: r
                .get::<Option<chrono::DateTime<Utc>>, _>("resolved_at")
                .map(|d| d.to_rfc3339()),
        })
        .collect();

    Ok(Json(serde_json::json!({ "results": items })))
}

// --- Recent comments ---

async fn get_recent_comments(
    State(state): State<Arc<AppState>>,
    Query(params): Query<LimitParam>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, String)> {
    let limit = params.limit.unwrap_or(20).clamp(1, 100);
    let rows = sqlx::query(
        "SELECT c.id, c.info_hash, t.name AS torrent_name, c.nickname, c.body, c.created_at \
         FROM torrent_comments c JOIN torrents t ON t.info_hash = c.info_hash \
         WHERE c.deleted = FALSE ORDER BY c.created_at DESC LIMIT $1",
    )
    .bind(limit)
    .fetch_all(&state.pool)
    .await
    .map_err(db_err)?;

    let comments: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            let body: String = r.get("body");
            let body_trimmed = if body.len() > 200 {
                format!("{}...", &body[..200])
            } else {
                body
            };
            serde_json::json!({
                "id": r.get::<i32, _>("id"),
                "info_hash": r.get::<String, _>("info_hash"),
                "torrent_name": r.get::<Option<String>, _>("torrent_name"),
                "nickname": r.get::<String, _>("nickname"),
                "body": body_trimmed,
                "created_at": r.get::<chrono::DateTime<Utc>, _>("created_at").to_rfc3339(),
            })
        })
        .collect();

    Ok(Json(serde_json::json!({ "comments": comments })))
}

// --- DHT status ---

async fn get_dht_status(
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

    let dht_running = state.settings.workers.iter().any(|w| w == "dht_crawler");

    Ok(Json(serde_json::json!({
        "dht_running": dht_running,
        "instances": if dht_running { state.settings.dht_instances } else { 0 },
        "routing_table_nodes": 0,
        "routing_table_good": 0,
        "hash_queue_size": 0,
        "total_hashes": total,
        "resolved": resolved,
        "unresolved": total - resolved,
        "resolve_rate": if total > 0 { resolved as f64 / total as f64 } else { 0.0 },
        "sync_enabled": state.settings.sync_enabled,
        "sync_sequence": 0,
        "sync_peers": state.settings.sync_peers.len(),
    })))
}

// --- Recent resolved ---

async fn get_recent(
    State(state): State<Arc<AppState>>,
    Query(params): Query<LimitParam>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, String)> {
    let limit = params.limit.unwrap_or(50).clamp(1, 100);
    let rows = sqlx::query(
        "SELECT t.info_hash, t.name, t.size, t.seed_count, t.peer_count, t.resolved_at, t.source, t.trackers, \
         c.content_type, c.resolution \
         FROM torrents t LEFT JOIN torrent_content c ON t.info_hash = c.info_hash \
         WHERE t.resolved_at IS NOT NULL AND t.announced_at IS NOT NULL AND t.seed_count >= 1 \
         ORDER BY t.resolved_at DESC LIMIT $1"
    )
    .bind(limit)
    .fetch_all(&state.pool)
    .await
    .map_err(db_err)?;

    let items: Vec<serde_json::Value> = rows.iter().map(|r| serde_json::json!({
        "info_hash": r.get::<String, _>("info_hash"),
        "name": r.get::<Option<String>, _>("name"),
        "size": r.get::<Option<i64>, _>("size"),
        "content_type": r.get::<Option<String>, _>("content_type"),
        "resolution": r.get::<Option<String>, _>("resolution"),
        "seed_count": r.get::<i32, _>("seed_count"),
        "peer_count": r.get::<i32, _>("peer_count"),
        "resolved_at": r.get::<Option<chrono::DateTime<Utc>>, _>("resolved_at").map(|d| d.to_rfc3339()),
        "source": r.get::<String, _>("source"),
        "trackers": r.get::<Option<serde_json::Value>, _>("trackers"),
    })).collect();

    Ok(Json(serde_json::json!({ "results": items })))
}

// --- Recently announced ---

async fn get_recently_announced(
    State(state): State<Arc<AppState>>,
    Query(params): Query<LimitParam>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, String)> {
    let limit = params.limit.unwrap_or(50).clamp(1, 100);
    let rows = sqlx::query(
        "SELECT t.info_hash, t.name, t.size, t.seed_count, t.peer_count, t.announced_at, t.source, \
         c.content_type, c.resolution \
         FROM torrents t LEFT JOIN torrent_content c ON t.info_hash = c.info_hash \
         WHERE t.announced_at IS NOT NULL \
         ORDER BY t.announced_at DESC LIMIT $1",
    )
    .bind(limit)
    .fetch_all(&state.pool)
    .await
    .map_err(db_err)?;

    let items: Vec<serde_json::Value> = rows.iter().map(|r| serde_json::json!({
        "info_hash": r.get::<String, _>("info_hash"),
        "name": r.get::<Option<String>, _>("name"),
        "size": r.get::<Option<i64>, _>("size"),
        "content_type": r.get::<Option<String>, _>("content_type"),
        "resolution": r.get::<Option<String>, _>("resolution"),
        "seed_count": r.get::<i32, _>("seed_count"),
        "peer_count": r.get::<i32, _>("peer_count"),
        "announced_at": r.get::<Option<chrono::DateTime<Utc>>, _>("announced_at").map(|d| d.to_rfc3339()),
        "source": r.get::<String, _>("source"),
    })).collect();

    Ok(Json(serde_json::json!({ "results": items })))
}

// --- Scraper status ---

async fn get_scraper_status(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, String)> {
    let rows = sqlx::query(
        "SELECT source, COUNT(*) as cnt FROM torrents WHERE source LIKE 'api:%' OR source LIKE 'web:%' OR source = 'uploaded' \
         GROUP BY source ORDER BY cnt DESC"
    )
    .fetch_all(&state.pool)
    .await
    .map_err(db_err)?;

    let source_counts: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "source": r.get::<String, _>("source"),
                "count": r.get::<i64, _>("cnt"),
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "running": false,
        "enabled": false,
        "source_counts": source_counts,
    })))
}

// --- Announcer status ---

async fn get_announcer_status(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, String)> {
    let pool = &state.pool;
    let announced_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM torrents WHERE announced_at IS NOT NULL")
            .fetch_one(pool)
            .await
            .map_err(db_err)?;
    let pending: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM torrents WHERE announced_at IS NULL AND name IS NOT NULL AND no_peers IS NOT TRUE"
    ).fetch_one(pool).await.map_err(db_err)?;

    let running =
        state.settings.workers.iter().any(|w| w == "announcer") && state.settings.announcer_enabled;

    Ok(Json(serde_json::json!({
        "running": running,
        "enabled": state.settings.announcer_enabled,
        "pool_size": state.settings.announcer_pool_size,
        "pool_active": 0,
        "pool_settled": 0,
        "announced_count": announced_count,
        "pending_announce_count": pending,
    })))
}

// --- Helpers ---

#[derive(Debug, Deserialize)]
pub struct LimitParam {
    limit: Option<i64>,
}

fn db_err(e: sqlx::Error) -> (axum::http::StatusCode, String) {
    tracing::error!(error = %e, "database error");
    (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/stats", get(get_stats))
        .route("/trending", get(get_trending))
        .route("/comments/recent", get(get_recent_comments))
        .route("/dht/status", get(get_dht_status))
        .route("/recent", get(get_recent))
        .route("/announced/recent", get(get_recently_announced))
        .route("/scraper/status", get(get_scraper_status))
        .route("/announcer/status", get(get_announcer_status))
}
