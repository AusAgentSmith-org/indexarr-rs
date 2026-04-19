use std::sync::Arc;

use axum::Router;
use axum::extract::{Path, State};
use axum::response::Json;
use axum::routing::get;
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::Row;

use crate::state::AppState;

#[derive(Debug, Serialize)]
struct TorrentDetail {
    info_hash: String,
    name: Option<String>,
    size: Option<i64>,
    piece_length: Option<i32>,
    piece_count: Option<i32>,
    private: bool,
    source: String,
    discovered_at: String,
    resolved_at: Option<String>,
    seed_count: i32,
    peer_count: i32,
    nfo: Option<String>,
    trackers: Option<serde_json::Value>,
    announced_at: Option<String>,
    epoch: i32,
    contributor_id: Option<String>,
    content: Option<serde_json::Value>,
    files: Vec<FileItem>,
    tags: Vec<String>,
    magnet_uri: String,
    vote_score: i64,
    comment_count: i64,
    nuke_count: i64,
    user_vote: Option<i32>,
}

#[derive(Debug, Serialize)]
struct FileItem {
    path: String,
    size: i64,
    extension: Option<String>,
}

async fn get_torrent(
    State(state): State<Arc<AppState>>,
    Path(info_hash): Path<String>,
) -> Result<Json<TorrentDetail>, (axum::http::StatusCode, String)> {
    let pool = &state.pool;
    let hash = info_hash.to_lowercase();

    // Fetch torrent
    let torrent_row = sqlx::query("SELECT * FROM torrents WHERE info_hash = $1")
        .bind(&hash)
        .fetch_optional(pool)
        .await
        .map_err(db_err)?;

    let row = torrent_row.ok_or_else(|| {
        (
            axum::http::StatusCode::NOT_FOUND,
            format!("torrent {hash} not found"),
        )
    })?;

    // Fetch content
    let content_row = sqlx::query("SELECT * FROM torrent_content WHERE info_hash = $1")
        .bind(&hash)
        .fetch_optional(pool)
        .await
        .map_err(db_err)?;

    let content = content_row.map(|r| {
        serde_json::json!({
            "content_type": r.get::<Option<String>, _>("content_type"),
            "title": r.get::<Option<String>, _>("title"),
            "year": r.get::<Option<i32>, _>("year"),
            "season": r.get::<Option<i32>, _>("season"),
            "episode": r.get::<Option<i32>, _>("episode"),
            "resolution": r.get::<Option<String>, _>("resolution"),
            "codec": r.get::<Option<String>, _>("codec"),
            "video_source": r.get::<Option<String>, _>("video_source"),
            "hdr": r.get::<Option<String>, _>("hdr"),
            "audio_codec": r.get::<Option<String>, _>("audio_codec"),
            "audio_channels": r.get::<Option<String>, _>("audio_channels"),
            "quality_score": r.get::<Option<i32>, _>("quality_score"),
            "language": r.get::<Option<String>, _>("language"),
            "group": r.get::<Option<String>, _>("group"),
            "platform": r.get::<Option<String>, _>("platform"),
            "is_anime": r.get::<bool, _>("is_anime"),
            "tmdb_id": r.get::<Option<i32>, _>("tmdb_id"),
            "imdb_id": r.get::<Option<String>, _>("imdb_id"),
        })
    });

    // Fetch files
    let file_rows = sqlx::query(
        "SELECT path, size, extension FROM torrent_files WHERE info_hash = $1 ORDER BY path",
    )
    .bind(&hash)
    .fetch_all(pool)
    .await
    .map_err(db_err)?;

    let files: Vec<FileItem> = file_rows
        .iter()
        .map(|r| FileItem {
            path: r.get("path"),
            size: r.get("size"),
            extension: r.get("extension"),
        })
        .collect();

    // Fetch tags
    let tag_rows =
        sqlx::query_scalar::<_, String>("SELECT tag FROM torrent_tags WHERE info_hash = $1")
            .bind(&hash)
            .fetch_all(pool)
            .await
            .map_err(db_err)?;

    // Vote score
    let vote_score: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(value), 0) FROM torrent_votes WHERE info_hash = $1",
    )
    .bind(&hash)
    .fetch_one(pool)
    .await
    .map_err(db_err)?;

    // Comment count
    let comment_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM torrent_comments WHERE info_hash = $1 AND deleted = FALSE",
    )
    .bind(&hash)
    .fetch_one(pool)
    .await
    .map_err(db_err)?;

    // Nuke count
    let nuke_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM nuke_suggestions WHERE info_hash = $1 AND reviewed = FALSE",
    )
    .bind(&hash)
    .fetch_one(pool)
    .await
    .map_err(db_err)?;

    // Build magnet URI
    let name = row.get::<Option<String>, _>("name");
    let info_hash_val: String = row.get("info_hash");
    let dn = name.as_deref().unwrap_or(&info_hash_val);
    let trackers_val: Option<serde_json::Value> = row.get("trackers");
    let tracker_params = match &trackers_val {
        Some(serde_json::Value::Array(arr)) => arr
            .iter()
            .filter_map(|v| v.as_str())
            .map(|t| format!("&tr={t}"))
            .collect::<String>(),
        _ => "&tr=udp://tracker.opentrackr.org:1337/announce&tr=udp://open.stealth.si:80/announce"
            .to_string(),
    };
    let magnet_uri = format!("magnet:?xt=urn:btih:{info_hash_val}&dn={dn}{tracker_params}");

    Ok(Json(TorrentDetail {
        info_hash: info_hash_val,
        name,
        size: row.get("size"),
        piece_length: row.get("piece_length"),
        piece_count: row.get("piece_count"),
        private: row.get("private"),
        source: row.get("source"),
        discovered_at: row.get::<DateTime<Utc>, _>("discovered_at").to_rfc3339(),
        resolved_at: row
            .get::<Option<DateTime<Utc>>, _>("resolved_at")
            .map(|d| d.to_rfc3339()),
        seed_count: row.get("seed_count"),
        peer_count: row.get("peer_count"),
        nfo: row.get("nfo"),
        trackers: trackers_val,
        announced_at: row
            .get::<Option<DateTime<Utc>>, _>("announced_at")
            .map(|d| d.to_rfc3339()),
        epoch: row.get("epoch"),
        contributor_id: row.get("contributor_id"),
        content,
        files,
        tags: tag_rows,
        magnet_uri,
        vote_score,
        comment_count,
        nuke_count,
        user_vote: None, // Would need fingerprint from request
    }))
}

fn db_err(e: sqlx::Error) -> (axum::http::StatusCode, String) {
    tracing::error!(error = %e, "database error");
    (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/torrent/{info_hash}", get(get_torrent))
}
