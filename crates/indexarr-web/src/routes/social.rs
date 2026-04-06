use std::sync::Arc;

use axum::Router;
use axum::extract::{Path, Query, State};
use axum::response::Json;
use axum::routing::{get, post};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;

use crate::state::AppState;

// --- Comments ---

#[derive(Debug, Deserialize)]
pub struct CreateCommentRequest {
    body: String,
    parent_id: Option<i32>,
    #[serde(default = "default_anon")]
    nickname: String,
}

fn default_anon() -> String { "Anonymous".into() }

#[derive(Debug, Serialize)]
struct CommentResponse {
    id: i32,
    parent_id: Option<i32>,
    nickname: String,
    body: String,
    created_at: String,
    edited_at: Option<String>,
    deleted: bool,
    is_own: bool,
}

async fn get_comments(
    State(state): State<Arc<AppState>>,
    Path(info_hash): Path<String>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, String)> {
    let hash = info_hash.to_lowercase();
    let rows = sqlx::query(
        "SELECT id, parent_id, nickname, body, created_at, edited_at, deleted, fingerprint FROM torrent_comments \
         WHERE info_hash = $1 ORDER BY created_at ASC"
    )
    .bind(&hash)
    .fetch_all(&state.pool)
    .await
    .map_err(db_err)?;

    let comments: Vec<CommentResponse> = rows.iter().map(|r| CommentResponse {
        id: r.get("id"),
        parent_id: r.get("parent_id"),
        nickname: r.get("nickname"),
        body: if r.get::<bool, _>("deleted") { "[deleted]".into() } else { r.get("body") },
        created_at: r.get::<DateTime<Utc>, _>("created_at").to_rfc3339(),
        edited_at: r.get::<Option<DateTime<Utc>>, _>("edited_at").map(|d| d.to_rfc3339()),
        deleted: r.get("deleted"),
        is_own: false, // Would need fingerprint from request
    }).collect();

    Ok(Json(serde_json::json!({
        "comments": comments,
        "total": comments.len(),
    })))
}

async fn create_comment(
    State(state): State<Arc<AppState>>,
    Path(info_hash): Path<String>,
    Json(body): Json<CreateCommentRequest>,
) -> Result<Json<CommentResponse>, (axum::http::StatusCode, String)> {
    let hash = info_hash.to_lowercase();

    // Validate body length
    if body.body.is_empty() || body.body.len() > 2000 {
        return Err((axum::http::StatusCode::BAD_REQUEST, "body must be 1-2000 chars".into()));
    }

    // Check torrent exists and is resolved
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM torrents WHERE info_hash = $1 AND resolved_at IS NOT NULL)"
    )
    .bind(&hash)
    .fetch_one(&state.pool)
    .await
    .map_err(db_err)?;

    if !exists {
        return Err((axum::http::StatusCode::NOT_FOUND, "torrent not found or unresolved".into()));
    }

    let fingerprint = "anonymous"; // In production, derive from IP/headers

    let row = sqlx::query(
        "INSERT INTO torrent_comments (info_hash, parent_id, nickname, body, fingerprint) \
         VALUES ($1, $2, $3, $4, $5) RETURNING id, created_at"
    )
    .bind(&hash)
    .bind(body.parent_id)
    .bind(&body.nickname)
    .bind(&body.body)
    .bind(fingerprint)
    .fetch_one(&state.pool)
    .await
    .map_err(db_err)?;

    Ok(Json(CommentResponse {
        id: row.get("id"),
        parent_id: body.parent_id,
        nickname: body.nickname,
        body: body.body,
        created_at: row.get::<DateTime<Utc>, _>("created_at").to_rfc3339(),
        edited_at: None,
        deleted: false,
        is_own: true,
    }))
}

// --- Votes ---

#[derive(Debug, Deserialize)]
pub struct VoteRequest {
    value: i32,
}

async fn get_votes(
    State(state): State<Arc<AppState>>,
    Path(info_hash): Path<String>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, String)> {
    let hash = info_hash.to_lowercase();

    let upvotes: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM torrent_votes WHERE info_hash = $1 AND value = 1"
    ).bind(&hash).fetch_one(&state.pool).await.map_err(db_err)?;

    let downvotes: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM torrent_votes WHERE info_hash = $1 AND value = -1"
    ).bind(&hash).fetch_one(&state.pool).await.map_err(db_err)?;

    Ok(Json(serde_json::json!({
        "upvotes": upvotes,
        "downvotes": downvotes,
        "score": upvotes - downvotes,
    })))
}

async fn cast_vote(
    State(state): State<Arc<AppState>>,
    Path(info_hash): Path<String>,
    Json(body): Json<VoteRequest>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, String)> {
    let hash = info_hash.to_lowercase();

    if body.value != 1 && body.value != -1 {
        return Err((axum::http::StatusCode::BAD_REQUEST, "value must be 1 or -1".into()));
    }

    let fingerprint = "anonymous";

    // Upsert: toggle if same vote, replace if different
    let existing = sqlx::query_scalar::<_, Option<i32>>(
        "SELECT value FROM torrent_votes WHERE info_hash = $1 AND fingerprint = $2"
    )
    .bind(&hash)
    .bind(fingerprint)
    .fetch_optional(&state.pool)
    .await
    .map_err(db_err)?
    .flatten();

    match existing {
        Some(v) if v == body.value => {
            // Toggle off
            sqlx::query("DELETE FROM torrent_votes WHERE info_hash = $1 AND fingerprint = $2")
                .bind(&hash).bind(fingerprint)
                .execute(&state.pool).await.map_err(db_err)?;
        }
        Some(_) => {
            // Change vote
            sqlx::query("UPDATE torrent_votes SET value = $1 WHERE info_hash = $2 AND fingerprint = $3")
                .bind(body.value).bind(&hash).bind(fingerprint)
                .execute(&state.pool).await.map_err(db_err)?;
        }
        None => {
            // New vote
            sqlx::query(
                "INSERT INTO torrent_votes (info_hash, fingerprint, value) VALUES ($1, $2, $3)"
            )
            .bind(&hash).bind(fingerprint).bind(body.value)
            .execute(&state.pool).await.map_err(db_err)?;
        }
    }

    // Return updated summary
    get_votes(State(state), Path(info_hash)).await
}

// --- Nuke Suggestions ---

#[derive(Debug, Deserialize)]
pub struct NukeSuggestRequest {
    reason: String,
}

async fn suggest_nuke(
    State(state): State<Arc<AppState>>,
    Path(info_hash): Path<String>,
    Json(body): Json<NukeSuggestRequest>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, String)> {
    let hash = info_hash.to_lowercase();

    if body.reason.len() < 10 || body.reason.len() > 500 {
        return Err((axum::http::StatusCode::BAD_REQUEST, "reason must be 10-500 chars".into()));
    }

    let fingerprint = "anonymous";

    sqlx::query(
        "INSERT INTO nuke_suggestions (info_hash, fingerprint, reason) VALUES ($1, $2, $3) \
         ON CONFLICT (info_hash, fingerprint) DO NOTHING"
    )
    .bind(&hash)
    .bind(fingerprint)
    .bind(&body.reason)
    .execute(&state.pool)
    .await
    .map_err(db_err)?;

    Ok(Json(serde_json::json!({ "status": "submitted" })))
}

async fn get_pending_nukes(
    State(state): State<Arc<AppState>>,
    Query(params): Query<LimitParam>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, String)> {
    let limit = params.limit.unwrap_or(50).clamp(1, 100);

    let rows = sqlx::query(
        "SELECT ns.id, ns.info_hash, t.name AS torrent_name, ns.reason, ns.created_at, \
         COUNT(*) OVER (PARTITION BY ns.info_hash) as suggestion_count \
         FROM nuke_suggestions ns JOIN torrents t ON t.info_hash = ns.info_hash \
         WHERE ns.reviewed = FALSE \
         ORDER BY suggestion_count DESC, ns.created_at DESC \
         LIMIT $1"
    )
    .bind(limit)
    .fetch_all(&state.pool)
    .await
    .map_err(db_err)?;

    let suggestions: Vec<serde_json::Value> = rows.iter().map(|r| serde_json::json!({
        "id": r.get::<i32, _>("id"),
        "info_hash": r.get::<String, _>("info_hash"),
        "torrent_name": r.get::<Option<String>, _>("torrent_name"),
        "reason": r.get::<String, _>("reason"),
        "created_at": r.get::<DateTime<Utc>, _>("created_at").to_rfc3339(),
        "suggestion_count": r.get::<i64, _>("suggestion_count"),
    })).collect();

    Ok(Json(serde_json::json!({
        "suggestions": suggestions,
        "total": suggestions.len(),
    })))
}

#[derive(Debug, Deserialize)]
pub struct ReviewNukeRequest {
    outcome: String,
}

async fn review_nuke(
    State(state): State<Arc<AppState>>,
    Path(suggestion_id): Path<i32>,
    Json(body): Json<ReviewNukeRequest>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, String)> {
    if !matches!(body.outcome.as_str(), "nuked" | "dismissed") {
        return Err((axum::http::StatusCode::BAD_REQUEST, "outcome must be 'nuked' or 'dismissed'".into()));
    }

    sqlx::query(
        "UPDATE nuke_suggestions SET reviewed = TRUE, reviewed_at = NOW(), outcome = $1 WHERE id = $2"
    )
    .bind(&body.outcome)
    .bind(suggestion_id)
    .execute(&state.pool)
    .await
    .map_err(db_err)?;

    // If nuked, delete the torrent
    if body.outcome == "nuked" {
        let hash: Option<String> = sqlx::query_scalar(
            "SELECT info_hash FROM nuke_suggestions WHERE id = $1"
        ).bind(suggestion_id).fetch_optional(&state.pool).await.map_err(db_err)?;

        if let Some(hash) = hash {
            sqlx::query("DELETE FROM torrents WHERE info_hash = $1")
                .bind(&hash)
                .execute(&state.pool)
                .await
                .map_err(db_err)?;
        }
    }

    Ok(Json(serde_json::json!({ "status": "reviewed" })))
}

async fn delete_comment(
    State(state): State<Arc<AppState>>,
    Path((_info_hash, comment_id)): Path<(String, i32)>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, String)> {
    // Soft delete — set deleted flag and replace body
    sqlx::query(
        "UPDATE torrent_comments SET deleted = TRUE, body = '[deleted]', edited_at = NOW() \
         WHERE id = $1"
    )
    .bind(comment_id)
    .execute(&state.pool)
    .await
    .map_err(db_err)?;

    Ok(Json(serde_json::json!({ "status": "deleted" })))
}

#[derive(Debug, Deserialize)]
struct LimitParam {
    limit: Option<i64>,
}

fn db_err(e: sqlx::Error) -> (axum::http::StatusCode, String) {
    tracing::error!(error = %e, "database error");
    (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
}

pub fn router() -> Router<Arc<AppState>> {
    use axum::routing::delete;
    Router::new()
        .route("/torrent/{info_hash}/comments", get(get_comments).post(create_comment))
        .route("/torrent/{info_hash}/comments/{comment_id}", delete(delete_comment))
        .route("/torrent/{info_hash}/votes", get(get_votes))
        .route("/torrent/{info_hash}/vote", post(cast_vote))
        .route("/torrent/{info_hash}/nuke", post(suggest_nuke))
        .route("/nuke/pending", get(get_pending_nukes))
        .route("/nuke/{suggestion_id}/review", post(review_nuke))
}
