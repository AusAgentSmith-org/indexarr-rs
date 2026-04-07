use std::sync::Arc;

use axum::Router;
use axum::extract::State;
use axum::response::Json;
use axum::routing::post;
use serde::{Deserialize, Serialize};

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
                    "SELECT EXISTS(SELECT 1 FROM torrents WHERE info_hash = $1)",
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
                         ON CONFLICT (info_hash) DO NOTHING",
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

    Ok(Json(MagnetUploadResponse {
        results,
        queued,
        duplicates,
        errors,
    }))
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

// --- .torrent file upload ---

async fn upload_torrents(
    State(state): State<Arc<AppState>>,
    mut multipart: axum::extract::Multipart,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, String)> {
    let pool = &state.pool;
    let mut results = Vec::new();
    let mut imported = 0i32;
    let mut duplicates = 0i32;
    let mut errors = 0i32;

    while let Ok(Some(field)) = multipart.next_field().await {
        let data = match field.bytes().await {
            Ok(d) => d,
            Err(e) => {
                errors += 1;
                results.push(serde_json::json!({
                    "info_hash": "", "name": null, "status": "error",
                    "message": format!("failed to read field: {e}")
                }));
                continue;
            }
        };

        if data.len() > 10 * 1024 * 1024 {
            errors += 1;
            results.push(serde_json::json!({
                "info_hash": "", "name": null, "status": "error",
                "message": "file too large (max 10MB)"
            }));
            continue;
        }

        match parse_torrent_file(&data) {
            Some(parsed) => {
                let exists: bool = sqlx::query_scalar(
                    "SELECT EXISTS(SELECT 1 FROM torrents WHERE info_hash = $1)",
                )
                .bind(&parsed.info_hash)
                .fetch_one(pool)
                .await
                .map_err(db_err)?;

                if exists {
                    duplicates += 1;
                    results.push(serde_json::json!({
                        "info_hash": parsed.info_hash, "name": parsed.name,
                        "status": "duplicate", "message": null
                    }));
                } else {
                    // Insert torrent
                    sqlx::query(
                        "INSERT INTO torrents (info_hash, name, size, source, resolved_at, private, piece_length, piece_count) \
                         VALUES ($1, $2, $3, 'upload', NOW(), $4, $5, $6) ON CONFLICT (info_hash) DO NOTHING"
                    )
                    .bind(&parsed.info_hash)
                    .bind(&parsed.name)
                    .bind(parsed.total_size)
                    .bind(parsed.is_private)
                    .bind(parsed.piece_length)
                    .bind(parsed.piece_count)
                    .execute(pool)
                    .await
                    .map_err(db_err)?;

                    // Insert files
                    for file in &parsed.files {
                        let ext = file.path.rsplit('.').next().map(|e| e.to_lowercase());
                        let _ = sqlx::query(
                            "INSERT INTO torrent_files (info_hash, path, size, extension) VALUES ($1, $2, $3, $4)"
                        )
                        .bind(&parsed.info_hash)
                        .bind(&file.path)
                        .bind(file.size)
                        .bind(&ext)
                        .execute(pool)
                        .await;
                    }

                    // Run content pipeline
                    let parsed_name = indexarr_parser::parse(&parsed.name);
                    let file_infos: Vec<indexarr_classifier::FileInfo> = parsed
                        .files
                        .iter()
                        .map(|f| indexarr_classifier::FileInfo {
                            path: f.path.clone(),
                            size: f.size,
                            extension: f.path.rsplit('.').next().map(|s| s.to_string()),
                        })
                        .collect();
                    let classification =
                        indexarr_classifier::classify(&parsed_name, &file_infos, &parsed.name);
                    let quality_score = indexarr_classifier::compute_quality_score(&parsed_name);

                    let _ = sqlx::query(
                        "INSERT INTO torrent_content (info_hash, content_type, title, year, season, episode, \
                         \"group\", resolution, codec, video_source, quality_score, classified_at, classifier_version) \
                         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, NOW(), '0.1.0') \
                         ON CONFLICT (info_hash) DO NOTHING"
                    )
                    .bind(&parsed.info_hash)
                    .bind(&classification.content_type)
                    .bind(&parsed_name.title)
                    .bind(parsed_name.year)
                    .bind(parsed_name.season)
                    .bind(parsed_name.episode)
                    .bind(&parsed_name.group)
                    .bind(&parsed_name.resolution)
                    .bind(&parsed_name.codec)
                    .bind(&parsed_name.video_source)
                    .bind(quality_score)
                    .execute(pool)
                    .await;

                    // Tags
                    for tag in &classification.tags {
                        let _ = sqlx::query(
                            "INSERT INTO torrent_tags (info_hash, tag, source) VALUES ($1, $2, 'classifier') \
                             ON CONFLICT (info_hash, tag) DO NOTHING"
                        )
                        .bind(&parsed.info_hash)
                        .bind(tag)
                        .execute(pool)
                        .await;
                    }

                    imported += 1;
                    results.push(serde_json::json!({
                        "info_hash": parsed.info_hash, "name": parsed.name,
                        "size": parsed.total_size, "content_type": classification.content_type,
                        "private": parsed.is_private, "status": "imported", "message": null
                    }));
                }
            }
            None => {
                errors += 1;
                results.push(serde_json::json!({
                    "info_hash": "", "name": null, "status": "error",
                    "message": "invalid .torrent file"
                }));
            }
        }
    }

    Ok(Json(serde_json::json!({
        "results": results, "imported": imported, "duplicates": duplicates, "errors": errors
    })))
}

struct ParsedTorrentFile {
    info_hash: String,
    name: String,
    total_size: i64,
    files: Vec<TorrentFileEntry>,
    is_private: bool,
    piece_length: Option<i32>,
    piece_count: Option<i32>,
}

struct TorrentFileEntry {
    path: String,
    size: i64,
}

/// Parse a .torrent file (bencode) and extract metadata.
fn parse_torrent_file(data: &[u8]) -> Option<ParsedTorrentFile> {
    let value = bencode_decode(data)?;
    let dict = value.as_dict()?;
    let info = dict.get("info")?.as_dict()?;

    // Compute info_hash = SHA1(bencoded info dict)
    let info_start = find_info_dict_range(data)?;
    use sha1::Digest;
    let mut hasher = sha1::Sha1::new();
    hasher.update(&data[info_start.0..info_start.1]);
    let info_hash = hex::encode(hasher.finalize());

    let name = info.get("name").and_then(|v| v.as_string())?.to_string();
    let piece_length = info
        .get("piece length")
        .and_then(|v| v.as_int())
        .map(|v| v as i32);
    let pieces_len = info
        .get("pieces")
        .and_then(|v| v.as_bytes())
        .map(|b| b.len())
        .unwrap_or(0);
    let piece_count = if pieces_len > 0 {
        Some((pieces_len / 20) as i32)
    } else {
        None
    };
    let is_private = info.get("private").and_then(|v| v.as_int()).unwrap_or(0) != 0;

    let mut files = Vec::new();
    let mut total_size: i64 = 0;

    if let Some(file_list) = info.get("files").and_then(|v| v.as_list()) {
        // Multi-file torrent
        for f in file_list {
            let fd = f.as_dict()?;
            let size = fd.get("length").and_then(|v| v.as_int()).unwrap_or(0);
            let path_parts = fd.get("path").and_then(|v| v.as_list())?;
            let path: String = path_parts
                .iter()
                .filter_map(|p| p.as_string())
                .collect::<Vec<_>>()
                .join("/");
            total_size += size;
            files.push(TorrentFileEntry { path, size });
        }
    } else {
        // Single-file torrent
        let size = info.get("length").and_then(|v| v.as_int()).unwrap_or(0);
        total_size = size;
        files.push(TorrentFileEntry {
            path: name.clone(),
            size,
        });
    }

    Some(ParsedTorrentFile {
        info_hash,
        name,
        total_size,
        files,
        is_private,
        piece_length,
        piece_count,
    })
}

// --- Minimal bencode parser ---

#[derive(Debug)]
enum BValue {
    Int(i64),
    Bytes(Vec<u8>),
    List(Vec<BValue>),
    Dict(Vec<(String, BValue)>),
}

impl BValue {
    fn as_int(&self) -> Option<i64> {
        if let BValue::Int(i) = self {
            Some(*i)
        } else {
            None
        }
    }
    fn as_bytes(&self) -> Option<&[u8]> {
        if let BValue::Bytes(b) = self {
            Some(b)
        } else {
            None
        }
    }
    fn as_string(&self) -> Option<&str> {
        self.as_bytes().and_then(|b| std::str::from_utf8(b).ok())
    }
    fn as_list(&self) -> Option<&[BValue]> {
        if let BValue::List(l) = self {
            Some(l)
        } else {
            None
        }
    }
    fn as_dict(&self) -> Option<&Vec<(String, BValue)>> {
        if let BValue::Dict(d) = self {
            Some(d)
        } else {
            None
        }
    }
}

trait DictExt {
    fn get(&self, key: &str) -> Option<&BValue>;
}

impl DictExt for Vec<(String, BValue)> {
    fn get(&self, key: &str) -> Option<&BValue> {
        self.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }
}

fn bencode_decode(data: &[u8]) -> Option<BValue> {
    let (val, _) = decode_value(data, 0, 0)?;
    Some(val)
}

fn decode_value(data: &[u8], pos: usize, depth: usize) -> Option<(BValue, usize)> {
    if depth > 32 || pos >= data.len() {
        return None;
    }
    match data[pos] {
        b'i' => decode_int(data, pos),
        b'l' => decode_list(data, pos, depth),
        b'd' => decode_dict(data, pos, depth),
        b'0'..=b'9' => decode_bytes(data, pos),
        _ => None,
    }
}

fn decode_int(data: &[u8], pos: usize) -> Option<(BValue, usize)> {
    let end = data[pos + 1..].iter().position(|&b| b == b'e')? + pos + 1;
    let s = std::str::from_utf8(&data[pos + 1..end]).ok()?;
    Some((BValue::Int(s.parse().ok()?), end + 1))
}

fn decode_bytes(data: &[u8], pos: usize) -> Option<(BValue, usize)> {
    let colon = data[pos..].iter().position(|&b| b == b':')? + pos;
    let len: usize = std::str::from_utf8(&data[pos..colon]).ok()?.parse().ok()?;
    let start = colon + 1;
    if start + len > data.len() {
        return None;
    }
    Some((
        BValue::Bytes(data[start..start + len].to_vec()),
        start + len,
    ))
}

fn decode_list(data: &[u8], pos: usize, depth: usize) -> Option<(BValue, usize)> {
    let mut items = Vec::new();
    let mut p = pos + 1;
    while p < data.len() && data[p] != b'e' {
        let (val, next) = decode_value(data, p, depth + 1)?;
        items.push(val);
        p = next;
    }
    Some((BValue::List(items), p + 1))
}

fn decode_dict(data: &[u8], pos: usize, depth: usize) -> Option<(BValue, usize)> {
    let mut items = Vec::new();
    let mut p = pos + 1;
    while p < data.len() && data[p] != b'e' {
        let (key_val, next) = decode_bytes(data, p)?;
        let key = String::from_utf8(if let BValue::Bytes(b) = key_val {
            b
        } else {
            return None;
        })
        .ok()?;
        let (val, next2) = decode_value(data, next, depth + 1)?;
        items.push((key, val));
        p = next2;
    }
    Some((BValue::Dict(items), p + 1))
}

/// Find the byte range of the "info" dict value in raw bencode.
fn find_info_dict_range(data: &[u8]) -> Option<(usize, usize)> {
    // Search for "4:info" key in the top-level dict
    let needle = b"4:infod";
    let pos = data.windows(needle.len()).position(|w| w == needle)?;
    let info_start = pos + 6; // after "4:info", at the 'd'

    // Find the matching end by counting dict/list nesting
    let mut depth = 0i32;
    let mut i = info_start;
    loop {
        if i >= data.len() {
            return None;
        }
        match data[i] {
            b'd' | b'l' => {
                depth += 1;
                i += 1;
            }
            b'e' => {
                depth -= 1;
                i += 1;
                if depth == 0 {
                    return Some((info_start, i));
                }
            }
            b'i' => {
                // Skip integer
                while i < data.len() && data[i] != b'e' {
                    i += 1;
                }
                i += 1; // skip 'e'
            }
            b'0'..=b'9' => {
                // Skip string: read length, skip colon + bytes
                let colon = data[i..].iter().position(|&b| b == b':')? + i;
                let len: usize = std::str::from_utf8(&data[i..colon]).ok()?.parse().ok()?;
                i = colon + 1 + len;
            }
            _ => return None,
        }
    }
}

fn db_err(e: sqlx::Error) -> (axum::http::StatusCode, String) {
    tracing::error!(error = %e, "database error");
    (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/upload", post(upload_torrents))
        .route("/upload/magnets", post(upload_magnets))
}
