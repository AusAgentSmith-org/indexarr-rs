use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::Mutex;

#[derive(Debug, Error)]
pub enum TmdbError {
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("api error: status {status}, body: {body}")]
    Api { status: u16, body: String },
    #[error("not configured (no API key)")]
    NotConfigured,
    #[error("circuit breaker open")]
    CircuitOpen,
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, TmdbError>;

/// TMDB search match result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TmdbMatch {
    pub tmdb_id: i32,
    pub media_type: String,
    pub title: String,
    pub original_title: Option<String>,
    pub year: Option<i32>,
    pub overview: Option<String>,
    pub vote_average: Option<f64>,
    pub vote_count: Option<i32>,
    pub genre_ids: Vec<i32>,
    pub popularity: Option<f64>,
    pub poster_path: Option<String>,
    pub backdrop_path: Option<String>,
}

/// TMDB full detail result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TmdbDetail {
    pub tmdb_id: i32,
    pub media_type: String,
    pub title: String,
    pub original_title: Option<String>,
    pub year: Option<i32>,
    pub overview: Option<String>,
    pub vote_average: Option<f64>,
    pub vote_count: Option<i32>,
    pub genres: Vec<Genre>,
    pub runtime: Option<i32>,
    pub status: Option<String>,
    pub tagline: Option<String>,
    pub imdb_id: Option<String>,
    pub number_of_seasons: Option<i32>,
    pub number_of_episodes: Option<i32>,
    pub cast: Vec<CastMember>,
    pub raw: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Genre {
    pub id: i32,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CastMember {
    pub name: String,
    pub character: Option<String>,
    pub order: Option<i32>,
}

/// TMDB API client with rate limiting and circuit breaker.
pub struct TmdbClient {
    api_key: String,
    client: reqwest::Client,
    rate_limit: f64,
    last_request: Mutex<Instant>,
    // Circuit breaker
    failure_count: AtomicU64,
    failure_threshold: u64,
    circuit_open_until: Mutex<Option<Instant>>,
    reset_timeout: Duration,
}

impl TmdbClient {
    pub fn new(api_key: &str, rate_limit: f64) -> Self {
        Self {
            api_key: api_key.to_string(),
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .unwrap(),
            rate_limit,
            last_request: Mutex::new(Instant::now() - Duration::from_secs(10)),
            failure_count: AtomicU64::new(0),
            failure_threshold: 5,
            circuit_open_until: Mutex::new(None),
            reset_timeout: Duration::from_secs(60),
        }
    }

    pub fn is_configured(&self) -> bool {
        !self.api_key.is_empty()
    }

    /// Search for movies by title and optional year.
    pub async fn search_movie(&self, query: &str, year: Option<i32>) -> Result<Vec<TmdbMatch>> {
        let mut url = format!(
            "https://api.themoviedb.org/3/search/movie?api_key={}&query={}",
            self.api_key,
            urlencoded(query)
        );
        if let Some(y) = year {
            url.push_str(&format!("&year={y}"));
        }
        let data: serde_json::Value = self.get(&url).await?;
        Ok(parse_search_results(&data, "movie"))
    }

    /// Search for TV shows by title and optional year.
    pub async fn search_tv(&self, query: &str, year: Option<i32>) -> Result<Vec<TmdbMatch>> {
        let mut url = format!(
            "https://api.themoviedb.org/3/search/tv?api_key={}&query={}",
            self.api_key,
            urlencoded(query)
        );
        if let Some(y) = year {
            url.push_str(&format!("&first_air_date_year={y}"));
        }
        let data: serde_json::Value = self.get(&url).await?;
        Ok(parse_search_results(&data, "tv"))
    }

    /// Get movie details including credits and external IDs.
    pub async fn movie_detail(&self, tmdb_id: i32) -> Result<TmdbDetail> {
        let url = format!(
            "https://api.themoviedb.org/3/movie/{tmdb_id}?api_key={}&append_to_response=credits,external_ids",
            self.api_key
        );
        let data: serde_json::Value = self.get(&url).await?;
        Ok(parse_detail(&data, "movie"))
    }

    /// Get TV show details including credits and external IDs.
    pub async fn tv_detail(&self, tmdb_id: i32) -> Result<TmdbDetail> {
        let url = format!(
            "https://api.themoviedb.org/3/tv/{tmdb_id}?api_key={}&append_to_response=credits,external_ids",
            self.api_key
        );
        let data: serde_json::Value = self.get(&url).await?;
        Ok(parse_detail(&data, "tv"))
    }

    /// Find by IMDB ID.
    pub async fn find_by_imdb(&self, imdb_id: &str) -> Result<Option<TmdbMatch>> {
        let url = format!(
            "https://api.themoviedb.org/3/find/{imdb_id}?api_key={}&external_source=imdb_id",
            self.api_key
        );
        let data: serde_json::Value = self.get(&url).await?;

        // Check movie_results first, then tv_results
        if let Some(results) = data.get("movie_results").and_then(|v| v.as_array())
            && let Some(first) = results.first()
        {
            return Ok(Some(parse_match(first, "movie")));
        }
        if let Some(results) = data.get("tv_results").and_then(|v| v.as_array())
            && let Some(first) = results.first()
        {
            return Ok(Some(parse_match(first, "tv")));
        }
        Ok(None)
    }

    async fn get(&self, url: &str) -> Result<serde_json::Value> {
        if !self.is_configured() {
            return Err(TmdbError::NotConfigured);
        }

        // Circuit breaker check
        {
            let open_until = self.circuit_open_until.lock().await;
            if let Some(until) = *open_until
                && Instant::now() < until
            {
                return Err(TmdbError::CircuitOpen);
            }
        }

        // Rate limiting
        {
            let mut last = self.last_request.lock().await;
            let min_interval = Duration::from_secs_f64(1.0 / self.rate_limit);
            let elapsed = last.elapsed();
            if elapsed < min_interval {
                tokio::time::sleep(min_interval - elapsed).await;
            }
            *last = Instant::now();
        }

        let resp = self.client.get(url).send().await.map_err(|e| {
            self.record_failure();
            TmdbError::Http(e)
        })?;

        let status = resp.status().as_u16();
        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            self.record_failure();
            return Err(TmdbError::Api { status, body });
        }

        // Success — reset failure count
        self.failure_count.store(0, Ordering::Relaxed);

        let data: serde_json::Value = resp.json().await?;
        Ok(data)
    }

    fn record_failure(&self) {
        let count = self.failure_count.fetch_add(1, Ordering::Relaxed) + 1;
        if count >= self.failure_threshold
            && let Ok(mut open_until) = self.circuit_open_until.try_lock()
        {
            *open_until = Some(Instant::now() + self.reset_timeout);
            tracing::warn!(
                failures = count,
                timeout_secs = self.reset_timeout.as_secs(),
                "TMDB circuit breaker opened"
            );
        }
    }
}

fn urlencoded(s: &str) -> String {
    s.replace(' ', "%20")
        .replace('&', "%26")
        .replace('=', "%3D")
}

fn parse_search_results(data: &serde_json::Value, media_type: &str) -> Vec<TmdbMatch> {
    data.get("results")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .map(|item| parse_match(item, media_type))
                .collect()
        })
        .unwrap_or_default()
}

fn parse_match(item: &serde_json::Value, media_type: &str) -> TmdbMatch {
    let title = item
        .get("title")
        .or_else(|| item.get("name"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let date_str = item
        .get("release_date")
        .or_else(|| item.get("first_air_date"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let year = if date_str.len() >= 4 {
        date_str[..4].parse().ok()
    } else {
        None
    };

    TmdbMatch {
        tmdb_id: item.get("id").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
        media_type: media_type.to_string(),
        title,
        original_title: item
            .get("original_title")
            .or_else(|| item.get("original_name"))
            .and_then(|v| v.as_str())
            .map(String::from),
        year,
        overview: item
            .get("overview")
            .and_then(|v| v.as_str())
            .map(String::from),
        vote_average: item.get("vote_average").and_then(|v| v.as_f64()),
        vote_count: item
            .get("vote_count")
            .and_then(|v| v.as_i64())
            .map(|v| v as i32),
        genre_ids: item
            .get("genre_ids")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_i64().map(|i| i as i32))
                    .collect()
            })
            .unwrap_or_default(),
        popularity: item.get("popularity").and_then(|v| v.as_f64()),
        poster_path: item
            .get("poster_path")
            .and_then(|v| v.as_str())
            .map(String::from),
        backdrop_path: item
            .get("backdrop_path")
            .and_then(|v| v.as_str())
            .map(String::from),
    }
}

fn parse_detail(data: &serde_json::Value, media_type: &str) -> TmdbDetail {
    let title = data
        .get("title")
        .or_else(|| data.get("name"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let date_str = data
        .get("release_date")
        .or_else(|| data.get("first_air_date"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let year = if date_str.len() >= 4 {
        date_str[..4].parse().ok()
    } else {
        None
    };

    let genres = data
        .get("genres")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|g| {
                    Some(Genre {
                        id: g.get("id")?.as_i64()? as i32,
                        name: g.get("name")?.as_str()?.to_string(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    // IMDB ID — direct field or from external_ids
    let imdb_id = data
        .get("imdb_id")
        .and_then(|v| v.as_str())
        .or_else(|| {
            data.get("external_ids")
                .and_then(|e| e.get("imdb_id"))
                .and_then(|v| v.as_str())
        })
        .map(String::from);

    // Cast — top 20
    let cast = data
        .get("credits")
        .and_then(|c| c.get("cast"))
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .take(20)
                .filter_map(|c| {
                    Some(CastMember {
                        name: c.get("name")?.as_str()?.to_string(),
                        character: c
                            .get("character")
                            .and_then(|v| v.as_str())
                            .map(String::from),
                        order: c.get("order").and_then(|v| v.as_i64()).map(|v| v as i32),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    TmdbDetail {
        tmdb_id: data.get("id").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
        media_type: media_type.to_string(),
        title,
        original_title: data
            .get("original_title")
            .or_else(|| data.get("original_name"))
            .and_then(|v| v.as_str())
            .map(String::from),
        year,
        overview: data
            .get("overview")
            .and_then(|v| v.as_str())
            .map(String::from),
        vote_average: data.get("vote_average").and_then(|v| v.as_f64()),
        vote_count: data
            .get("vote_count")
            .and_then(|v| v.as_i64())
            .map(|v| v as i32),
        genres,
        runtime: data
            .get("runtime")
            .and_then(|v| v.as_i64())
            .map(|v| v as i32),
        status: data
            .get("status")
            .and_then(|v| v.as_str())
            .map(String::from),
        tagline: data
            .get("tagline")
            .and_then(|v| v.as_str())
            .map(String::from),
        imdb_id,
        number_of_seasons: data
            .get("number_of_seasons")
            .and_then(|v| v.as_i64())
            .map(|v| v as i32),
        number_of_episodes: data
            .get("number_of_episodes")
            .and_then(|v| v.as_i64())
            .map(|v| v as i32),
        cast,
        raw: data.clone(),
    }
}
