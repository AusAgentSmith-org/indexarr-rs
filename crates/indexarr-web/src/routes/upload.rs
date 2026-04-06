use std::sync::Arc;

use axum::Router;
use axum::extract::State;
use axum::response::Json;
use axum::routing::post;
use serde::{Deserialize, Serialize};
use sqlx::Row;

use crate::state::AppState;

// --- Magnet Upload ---

#[derive(Debug, Deserialize)]
pub struct MagnetUploadRequest {
    magnets: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct MagnetResult {
    info_hash: String,
    name: Option<String>,
    status: String,
    message: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct MagnetUploadResponse {
    results: Vec<MagnetResult>,
    queued: i32,
    duplicates: i32,
    errors: i32,
}

async fn upload_magnets(
    State(state): State<Arc<AppState>>,
    Json(body): Json<MagnetUploadRequest>,
) -> Result<Json<MagnetUploadResponse>, (axum::http::StatusCode, String)> {
    let pool = &state.pool;
    let mut results = Vec::new();
    let mut queued = 0i32;
    let mut duplicates = 0i32;
    let mut errors = 0i32;

    for magnet in &body.magnets {
        match parse_magnet(magnet) {
            Some((hash, name)) => {
                // Check for existing
                let exists: bool = sqlx::query_scalar(
                    "SELECT EXISTS(SELECT 1 FROM torrents WHERE info_hash = $1)"
                )
                .bind(&hash)
                .fetch_one(pool)
                .await
                .map_err(db_err)?;

                if exists {
                    duplicates += 1;
                    results.push(MagnetResult {
                        info_hash: hash,
                        name,
                        status: "duplicate".into(),
                        message: None,
                    });
                } else {
                    sqlx::query(
                        "INSERT INTO torrents (info_hash, name, source, priority, discovered_at) \
                         VALUES ($1, $2, 'upload', TRUE, NOW()) \
                         ON CONFLICT (info_hash) DO NOTHING"
                    )
                    .bind(&hash)
                    .bind(&name)
                    .execute(pool)
                    .await
                    .map_err(db_err)?;

                    queued += 1;
                    results.push(MagnetResult {
                        info_hash: hash,
                        name,
                        status: "queued".into(),
                        message: None,
                    });
                }
            }
            None => {
                errors += 1;
                results.push(MagnetResult {
                    info_hash: String::new(),
                    name: None,
                    status: "error".into(),
                    message: Some("invalid magnet URI".into()),
                });
            }
        }
    }

    Ok(Json(MagnetUploadResponse { results, queued, duplicates, errors }))
}

/// Parse a magnet URI and extract (info_hash, display_name).
fn parse_magnet(uri: &str) -> Option<(String, Option<String>)> {
    if !uri.starts_with("magnet:?") {
        return None;
    }

    let query = &uri[8..];
    let mut info_hash = None;
    let mut display_name = None;

    for param in query.split('&') {
        if let Some((key, value)) = param.split_once('=') {
            match key {
                "xt" => {
                    // urn:btih:<hash>
                    if let Some(hash_part) = value.strip_prefix("urn:btih:") {
                        let hash = if hash_part.len() == 40 {
                            // Hex
                            hash_part.to_lowercase()
                        } else if hash_part.len() == 32 {
                            // Base32 → hex
                            base32_to_hex(hash_part).unwrap_or_default()
                        } else {
                            continue;
                        };
                        if hash.len() == 40 {
                            info_hash = Some(hash);
                        }
                    }
                }
                "dn" => {
                    display_name = Some(urldecode(value));
                }
                _ => {}
            }
        }
    }

    info_hash.map(|h| (h, display_name))
}

fn base32_to_hex(input: &str) -> Option<String> {
    let upper = input.to_uppercase();
    let decoded = base32::decode(base32::Alphabet::Rfc4648 { padding: false }, &upper)?;
    Some(hex::encode(decoded))
}

fn urldecode(s: &str) -> String {
    s.replace('+', " ")
        .replace("%20", " ")
        .replace("%26", "&")
        .replace("%3D", "=")
}

fn db_err(e: sqlx::Error) -> (axum::http::StatusCode, String) {
    tracing::error!(error = %e, "database error");
    (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/upload/magnets", post(upload_magnets))
}
