use std::sync::Arc;

use axum::Router;
use axum::extract::{Query, State};
use axum::response::Json;
use axum::routing::get;
use serde::{Deserialize, Serialize};

use crate::state::AppState;
use indexarr_search::{SearchFilters, SortField, SortOrder};

#[derive(Debug, Deserialize)]
pub struct SearchParams {
    #[serde(default)]
    q: String,
    content_type: Option<String>,
    resolution: Option<String>,
    codec: Option<String>,
    video_source: Option<String>,
    hdr: Option<String>,
    audio_codec: Option<String>,
    modifier: Option<String>,
    year: Option<i32>,
    year_min: Option<i32>,
    year_max: Option<i32>,
    language: Option<String>,
    tag: Option<String>,
    imdb_id: Option<String>,
    tmdb_id: Option<i32>,
    season: Option<i32>,
    episode: Option<i32>,
    min_seeders: Option<i32>,
    platform: Option<String>,
    has_subtitles: Option<bool>,
    music_format: Option<String>,
    network: Option<String>,
    edition: Option<String>,
    category: Option<i32>,
    source: Option<String>,
    #[serde(default = "default_sort")]
    sort: String,
    #[serde(default = "default_order")]
    order: String,
    #[serde(default = "default_limit")]
    limit: i64,
    #[serde(default)]
    offset: i64,
    #[serde(default = "default_true")]
    facets: bool,
}

fn default_sort() -> String { "relevance".into() }
fn default_order() -> String { "desc".into() }
fn default_limit() -> i64 { 50 }
fn default_true() -> bool { true }

#[derive(Debug, Serialize)]
pub struct SearchResultItem {
    info_hash: String,
    name: Option<String>,
    size: Option<i64>,
    content_type: Option<String>,
    resolution: Option<String>,
    codec: Option<String>,
    video_source: Option<String>,
    year: Option<i32>,
    season: Option<i32>,
    episode: Option<i32>,
    seed_count: i32,
    peer_count: i32,
    discovered_at: Option<String>,
    resolved_at: Option<String>,
    tags: Vec<String>,
    audio_codec: Option<String>,
    hdr: Option<String>,
    platform: Option<String>,
    quality_score: Option<i32>,
    network: Option<String>,
    edition: Option<String>,
    trackers: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct FacetValue {
    value: String,
    count: i64,
}

#[derive(Debug, Serialize)]
pub struct SearchFacetsResponse {
    content_type: Vec<FacetValue>,
    resolution: Vec<FacetValue>,
    codec: Vec<FacetValue>,
    video_source: Vec<FacetValue>,
    hdr: Vec<FacetValue>,
    year: Vec<FacetValue>,
    source: Vec<FacetValue>,
    audio_codec: Vec<FacetValue>,
    language: Vec<FacetValue>,
    modifier: Vec<FacetValue>,
    platform: Vec<FacetValue>,
    music_format: Vec<FacetValue>,
}

#[derive(Debug, Serialize)]
pub struct SearchApiResponse {
    results: Vec<SearchResultItem>,
    total: i64,
    offset: i64,
    limit: i64,
    facets: Option<SearchFacetsResponse>,
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/search", get(search_torrents))
}

async fn search_torrents(
    State(state): State<Arc<AppState>>,
    Query(params): Query<SearchParams>,
) -> Result<Json<SearchApiResponse>, (axum::http::StatusCode, String)> {
    let filters = SearchFilters {
        query: params.q,
        content_type: params.content_type,
        resolution: params.resolution,
        codec: params.codec,
        video_source: params.video_source,
        modifier: params.modifier,
        hdr: params.hdr,
        audio_codec: params.audio_codec,
        year: params.year,
        year_min: params.year_min,
        year_max: params.year_max,
        language: params.language,
        tag: params.tag,
        imdb_id: params.imdb_id,
        tmdb_id: params.tmdb_id,
        season: params.season,
        episode: params.episode,
        source: params.source,
        platform: params.platform,
        has_subtitles: params.has_subtitles,
        music_format: params.music_format,
        network: params.network,
        edition: params.edition,
        category: params.category,
        min_seeders: params.min_seeders,
        sort: SortField::from_str_loose(&params.sort),
        order: SortOrder::from_str_loose(&params.order),
        limit: params.limit.clamp(1, 100),
        offset: params.offset.max(0),
        ..Default::default()
    };

    let response = indexarr_search::search(&state.pool, &filters, params.facets)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "search query failed");
            (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        })?;

    let items: Vec<SearchResultItem> = response
        .results
        .iter()
        .map(|r| {
            let c = r.content.as_ref();
            SearchResultItem {
                info_hash: r.torrent.info_hash.clone(),
                name: r.torrent.name.clone(),
                size: r.torrent.size,
                content_type: c.and_then(|c| c.content_type.clone()),
                resolution: c.and_then(|c| c.resolution.clone()),
                codec: c.and_then(|c| c.codec.clone()),
                video_source: c.and_then(|c| c.video_source.clone()),
                year: c.and_then(|c| c.year),
                season: c.and_then(|c| c.season),
                episode: c.and_then(|c| c.episode),
                seed_count: r.torrent.seed_count,
                peer_count: r.torrent.peer_count,
                discovered_at: r.torrent.discovered_at.to_rfc3339().into(),
                resolved_at: r.torrent.resolved_at.map(|d| d.to_rfc3339()),
                tags: r.tags.clone(),
                audio_codec: c.and_then(|c| c.audio_codec.clone()),
                hdr: c.and_then(|c| c.hdr.clone()),
                platform: c.and_then(|c| c.platform.clone()),
                quality_score: c.and_then(|c| c.quality_score),
                network: c.and_then(|c| c.network.clone()),
                edition: c.and_then(|c| c.edition.clone()),
                trackers: r.torrent.trackers.clone(),
            }
        })
        .collect();

    let facets_resp = response.facets.map(|f| {
        let conv = |v: Vec<indexarr_search::FacetCount>| -> Vec<FacetValue> {
            v.into_iter()
                .map(|fc| FacetValue {
                    value: fc.value,
                    count: fc.count,
                })
                .collect()
        };
        SearchFacetsResponse {
            content_type: conv(f.content_type),
            resolution: conv(f.resolution),
            codec: conv(f.codec),
            video_source: conv(f.video_source),
            hdr: conv(f.hdr),
            year: conv(f.year),
            source: conv(f.source),
            audio_codec: conv(f.audio_codec),
            language: conv(f.language),
            modifier: conv(f.modifier),
            platform: conv(f.platform),
            music_format: conv(f.music_format),
        }
    });

    Ok(Json(SearchApiResponse {
        results: items,
        total: response.total,
        offset: response.offset,
        limit: response.limit,
        facets: facets_resp,
    }))
}
