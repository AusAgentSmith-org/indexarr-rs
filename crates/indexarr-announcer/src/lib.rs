use std::collections::HashMap;
use std::time::Instant;

use chrono::{DateTime, Utc};
use sqlx::{PgPool, Row};
use tokio_util::sync::CancellationToken;

use indexarr_core::config::Settings;

/// Tracked torrent in the announcer pool.
struct TrackedHandle {
    info_hash: String,
    name: String,
    trackers: Vec<String>,
    added_at: Instant,
    best_seeds: i32,
    best_peers: i32,
    settled: bool,
    announce_miss: i32,
}

/// Rolling pool announcer — validates seed/peer counts for resolved torrents.
///
/// Uses HTTP tracker scrape to get seed/peer counts. Falls back to DHT-reported
/// counts. Implements 3-strike rule: after 3 consecutive zero-activity scrapes,
/// marks the torrent as no_peers.
pub struct TorrentAnnouncer {
    pool: PgPool,
    settings: AnnouncerConfig,
    tracked: HashMap<String, TrackedHandle>,
    announced_count: u64,
    start_time: Instant,
}

#[derive(Clone)]
pub struct AnnouncerConfig {
    pub pool_size: u32,
    pub poll_interval: u64,
    pub settle_time: u64,
    pub rotate_interval: u64,
    pub default_trackers: Vec<String>,
}

impl AnnouncerConfig {
    pub fn from_settings(settings: &Settings) -> Self {
        Self {
            pool_size: settings.announcer_pool_size,
            poll_interval: settings.announcer_poll_interval,
            settle_time: settings.announcer_settle_time,
            rotate_interval: settings.announcer_rotate_interval,
            default_trackers: settings.default_trackers.clone(),
        }
    }
}

impl TorrentAnnouncer {
    pub fn new(pool: PgPool, config: AnnouncerConfig) -> Self {
        Self {
            pool,
            settings: config,
            tracked: HashMap::new(),
            announced_count: 0,
            start_time: Instant::now(),
        }
    }

    /// Main announcer loop.
    pub async fn run(&mut self, cancel: CancellationToken) {
        tracing::info!(pool_size = self.settings.pool_size, "announcer started");

        loop {
            if cancel.is_cancelled() { break; }

            // 1. Backfill pool with candidates from DB
            if let Err(e) = self.load_candidates().await {
                tracing::error!(error = %e, "failed to load candidates");
            }

            // 2. Poll and harvest settled torrents
            let harvested = self.poll_and_harvest().await;

            // 3. Persist results to DB
            if !harvested.is_empty() {
                if let Err(e) = self.persist_results(&harvested).await {
                    tracing::error!(error = %e, "failed to persist announcer results");
                }
            }

            // 4. Remove harvested from pool
            for (hash, _) in &harvested {
                self.tracked.remove(hash);
            }

            // Stats
            if self.announced_count % 100 == 0 && self.announced_count > 0 {
                tracing::info!(
                    announced = self.announced_count,
                    pool = self.tracked.len(),
                    uptime_secs = self.start_time.elapsed().as_secs(),
                    "announcer stats"
                );
            }

            tokio::time::sleep(std::time::Duration::from_secs(self.settings.poll_interval)).await;
        }

        tracing::info!(total_announced = self.announced_count, "announcer stopped");
    }

    /// Fill pool slots from DB with unannounced/stale torrents.
    async fn load_candidates(&mut self) -> Result<(), sqlx::Error> {
        let free_slots = self.settings.pool_size as usize - self.tracked.len();
        if free_slots == 0 { return Ok(()); }

        let exclude: Vec<&str> = self.tracked.keys().map(|s| s.as_str()).collect();

        // Query candidates: resolved, not no_peers, not already in pool
        let rows = sqlx::query(
            "SELECT info_hash, name, trackers FROM torrents \
             WHERE name IS NOT NULL AND no_peers IS NOT TRUE \
               AND info_hash != ALL($1) \
             ORDER BY announced_at ASC NULLS FIRST \
             LIMIT $2"
        )
        .bind(&exclude)
        .bind(free_slots as i64)
        .fetch_all(&self.pool)
        .await?;

        for row in &rows {
            let hash: String = row.get("info_hash");
            let name: String = row.get::<Option<String>, _>("name").unwrap_or_default();
            let trackers_json: Option<serde_json::Value> = row.get("trackers");
            let trackers = parse_trackers(trackers_json, &self.settings.default_trackers);

            self.tracked.insert(hash.clone(), TrackedHandle {
                info_hash: hash,
                name,
                trackers,
                added_at: Instant::now(),
                best_seeds: 0,
                best_peers: 0,
                settled: false,
                announce_miss: 0,
            });
        }

        Ok(())
    }

    /// Poll tracker scrape for all tracked torrents and harvest settled ones.
    async fn poll_and_harvest(&mut self) -> Vec<(String, HarvestResult)> {
        let mut harvested = Vec::new();
        let now = Instant::now();

        let hashes: Vec<String> = self.tracked.keys().cloned().collect();
        for hash in hashes {
            let handle = match self.tracked.get_mut(&hash) {
                Some(h) => h,
                None => continue,
            };

            let age = now.duration_since(handle.added_at).as_secs();

            // Mark settled
            if age >= self.settings.settle_time && !handle.settled {
                handle.settled = true;
            }

            // Scrape tracker for seed/peer counts
            let (seeds, peers) = scrape_trackers(&handle.info_hash, &handle.trackers).await;
            handle.best_seeds = handle.best_seeds.max(seeds);
            handle.best_peers = handle.best_peers.max(peers);

            // Harvest if past rotate interval
            if age >= self.settings.rotate_interval {
                harvested.push((hash.clone(), HarvestResult {
                    seeds: handle.best_seeds,
                    peers: handle.best_peers,
                }));
            }
        }

        harvested
    }

    /// Persist harvest results to DB.
    async fn persist_results(&mut self, results: &[(String, HarvestResult)]) -> Result<(), sqlx::Error> {
        for (hash, result) in results {
            let has_activity = result.seeds > 0 || result.peers > 0;

            if has_activity {
                // Update with new counts
                sqlx::query(
                    "UPDATE torrents SET \
                       seed_count = GREATEST(seed_count, $2), \
                       peer_count = GREATEST(peer_count, $3), \
                       announced_at = NOW(), \
                       announce_miss = 0 \
                     WHERE info_hash = $1"
                )
                .bind(hash)
                .bind(result.seeds)
                .bind(result.peers)
                .execute(&self.pool)
                .await?;
            } else {
                // 3-strike rule
                let miss: Option<i32> = sqlx::query_scalar(
                    "UPDATE torrents SET announce_miss = announce_miss + 1, announced_at = NOW() \
                     WHERE info_hash = $1 RETURNING announce_miss"
                )
                .bind(hash)
                .fetch_optional(&self.pool)
                .await?;

                if let Some(miss_count) = miss {
                    if miss_count >= 3 {
                        sqlx::query("UPDATE torrents SET no_peers = TRUE WHERE info_hash = $1")
                            .bind(hash)
                            .execute(&self.pool)
                            .await?;
                        tracing::debug!(hash = %hash, misses = miss_count, "marked no_peers (3-strike)");
                    }
                }
            }

            self.announced_count += 1;
        }
        Ok(())
    }
}

struct HarvestResult {
    seeds: i32,
    peers: i32,
}

/// Parse tracker URLs from JSON or use defaults.
fn parse_trackers(json: Option<serde_json::Value>, defaults: &[String]) -> Vec<String> {
    if let Some(serde_json::Value::Array(arr)) = json {
        let trackers: Vec<String> = arr.iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect();
        if !trackers.is_empty() {
            return trackers;
        }
    }
    defaults.to_vec()
}

/// Scrape tracker(s) for seed/peer counts of an info_hash.
///
/// This is a simplified implementation using HTTP tracker scrape protocol.
/// For UDP trackers, a full BEP 15 implementation would be needed.
/// Falls back to (0, 0) if no tracker responds.
async fn scrape_trackers(info_hash: &str, trackers: &[String]) -> (i32, i32) {
    let Ok(hash_bytes) = hex::decode(info_hash) else { return (0, 0) };
    if hash_bytes.len() != 20 { return (0, 0); }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap_or_default();

    for tracker_url in trackers {
        // Only HTTP trackers for now (UDP tracker BEP 15 scrape deferred)
        if !tracker_url.starts_with("http") { continue; }

        // Convert announce URL to scrape URL
        let scrape_url = tracker_url.replace("/announce", "/scrape");
        let url = format!("{scrape_url}?info_hash={}", url_encode_hash(&hash_bytes));

        match client.get(&url).send().await {
            Ok(resp) if resp.status().is_success() => {
                if let Ok(body) = resp.bytes().await {
                    if let Some((seeds, peers)) = parse_scrape_response(&body, &hash_bytes) {
                        return (seeds, peers);
                    }
                }
            }
            _ => continue,
        }
    }

    (0, 0)
}

/// URL-encode a 20-byte info_hash for tracker scrape.
fn url_encode_hash(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("%{b:02x}")).collect()
}

/// Parse a bencoded scrape response for seed/peer counts.
fn parse_scrape_response(data: &[u8], info_hash: &[u8]) -> Option<(i32, i32)> {
    // Find the info_hash in the raw bytes
    let hash_pos = data.windows(20).position(|w| w == info_hash)?;
    let after = &data[hash_pos + 20..];

    // Extract integers from the bencode after the hash
    let seeds = extract_bencode_int_bytes(after, b"8:complete")?;
    let peers = extract_bencode_int_bytes(after, b"10:incomplete").unwrap_or(0);

    Some((seeds, peers))
}

fn extract_bencode_int_bytes(data: &[u8], key: &[u8]) -> Option<i32> {
    let pos = data.windows(key.len()).position(|w| w == key)?;
    let after_key = &data[pos + key.len()..];
    // Expect 'i' <digits> 'e'
    if after_key.first() != Some(&b'i') { return None; }
    let end = after_key.iter().position(|&b| b == b'e')?;
    std::str::from_utf8(&after_key[1..end]).ok()?.parse().ok()
}
