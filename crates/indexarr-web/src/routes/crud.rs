use std::sync::Arc;

use axum::Router;
use axum::extract::{Query, State};
use axum::response::Json;
use axum::routing::{delete, post};
use serde::Deserialize;

use crate::state::AppState;

// --- Tag CRUD ---

#[derive(Debug, Deserialize)]
pub struct TagRequest {
    info_hashes: Vec<String>,
    tags: Vec<String>,
}

async fn add_tags(
    State(state): State<Arc<AppState>>,
    Json(body): Json<TagRequest>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, String)> {
    let mut affected = 0i64;
    for hash in &body.info_hashes {
        for tag in &body.tags {
            let result = sqlx::query(
                "INSERT INTO torrent_tags (info_hash, tag, source) VALUES ($1, $2, 'user') \
                 ON CONFLICT (info_hash, tag) DO NOTHING",
            )
            .bind(hash)
            .bind(tag)
            .execute(&state.pool)
            .await
            .map_err(db_err)?;
            affected += result.rows_affected() as i64;
        }
    }
    Ok(Json(serde_json::json!({ "affected": affected })))
}

async fn remove_tags(
    State(state): State<Arc<AppState>>,
    Json(body): Json<TagRequest>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, String)> {
    let mut affected = 0i64;
    for hash in &body.info_hashes {
        for tag in &body.tags {
            let result = sqlx::query("DELETE FROM torrent_tags WHERE info_hash = $1 AND tag = $2")
                .bind(hash)
                .bind(tag)
                .execute(&state.pool)
                .await
                .map_err(db_err)?;
            affected += result.rows_affected() as i64;
        }
    }
    Ok(Json(serde_json::json!({ "affected": affected })))
}

// --- Bulk Delete ---

#[derive(Debug, Deserialize)]
pub struct DeleteParams {
    info_hashes: String,
}

async fn delete_torrents(
    State(state): State<Arc<AppState>>,
    Query(params): Query<DeleteParams>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, String)> {
    let hashes: Vec<&str> = params.info_hashes.split(',').map(|s| s.trim()).collect();
    let result = sqlx::query("DELETE FROM torrents WHERE info_hash = ANY($1)")
        .bind(&hashes)
        .execute(&state.pool)
        .await
        .map_err(db_err)?;
    Ok(Json(
        serde_json::json!({ "affected": result.rows_affected() }),
    ))
}

// --- Bulk Import ---

#[derive(Debug, Deserialize)]
pub struct ImportItem {
    info_hash: String,
    name: Option<String>,
    size: Option<i64>,
    #[serde(default = "default_import_source")]
    source: String,
    #[serde(default)]
    files: Vec<ImportFile>,
    trackers: Option<Vec<String>>,
    nfo: Option<String>,
    seed_count: Option<i32>,
    peer_count: Option<i32>,
    discovered_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct ImportFile {
    path: String,
    size: i64,
}

fn default_import_source() -> String {
    "import".to_string()
}

async fn import_torrents(
    State(state): State<Arc<AppState>>,
    Json(items): Json<Vec<ImportItem>>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, String)> {
    let pool = &state.pool;
    let mut imported = 0i64;
    let mut skipped = 0i64;

    for item in &items {
        let hash = item.info_hash.to_lowercase();
        if hash.len() != 40 {
            skipped += 1;
            continue;
        }

        // Check duplicate
        let exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM torrents WHERE info_hash = $1)")
                .bind(&hash)
                .fetch_one(pool)
                .await
                .map_err(db_err)?;

        if exists {
            skipped += 1;
            continue;
        }

        // Insert torrent
        let trackers_json = item
            .trackers
            .as_ref()
            .filter(|t| !t.is_empty())
            .map(|t| serde_json::to_value(t).unwrap_or(serde_json::Value::Null));
        sqlx::query(
            "INSERT INTO torrents \
             (info_hash, name, size, source, resolved_at, trackers, nfo, \
              seed_count, peer_count, discovered_at) \
             VALUES ($1, $2, $3, $4, \
                     CASE WHEN $2 IS NOT NULL THEN NOW() ELSE NULL END, \
                     $5, $6, COALESCE($7, 0), COALESCE($8, 0), \
                     COALESCE($9, NOW())) \
             ON CONFLICT (info_hash) DO NOTHING",
        )
        .bind(&hash)
        .bind(&item.name)
        .bind(item.size)
        .bind(&item.source)
        .bind(&trackers_json)
        .bind(&item.nfo)
        .bind(item.seed_count)
        .bind(item.peer_count)
        .bind(item.discovered_at)
        .execute(pool)
        .await
        .map_err(db_err)?;

        // Insert files
        for file in &item.files {
            let ext = file.path.rsplit('.').next().map(|e| e.to_lowercase());
            sqlx::query(
                "INSERT INTO torrent_files (info_hash, path, size, extension) VALUES ($1, $2, $3, $4)"
            )
            .bind(&hash)
            .bind(&file.path)
            .bind(file.size)
            .bind(&ext)
            .execute(pool)
            .await
            .map_err(db_err)?;
        }

        imported += 1;
    }

    Ok(Json(
        serde_json::json!({ "imported": imported, "skipped": skipped }),
    ))
}

fn db_err(e: sqlx::Error) -> (axum::http::StatusCode, String) {
    tracing::error!(error = %e, "database error");
    (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/torrents/tags", post(add_tags).delete(remove_tags))
        .route("/torrents", delete(delete_torrents))
        .route("/import", post(import_torrents))
}
