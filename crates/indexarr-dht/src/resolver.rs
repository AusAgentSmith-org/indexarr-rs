use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;

use sqlx::{PgPool, Row};
use tokio_util::sync::CancellationToken;

use crate::DhtSharedState;
use crate::engine::DhtEngine;

/// Metadata resolver — fetches torrent metadata via BEP 9 and runs content pipeline.
pub struct MetadataResolver {
    pool: PgPool,
    shared: Arc<DhtSharedState>,
    engine: Arc<DhtEngine>,
    workers: usize,
    timeout_secs: u64,
    save_files_threshold: u32,
    cancel: CancellationToken,
}

impl MetadataResolver {
    pub fn new(
        pool: PgPool,
        shared: Arc<DhtSharedState>,
        engine: Arc<DhtEngine>,
        workers: usize,
        timeout_secs: u64,
        save_files_threshold: u32,
        cancel: CancellationToken,
    ) -> Self {
        Self {
            pool,
            shared,
            engine,
            workers,
            timeout_secs,
            save_files_threshold,
            cancel,
        }
    }

    /// Main resolver loop.
    pub async fn run(&self) {
        tracing::info!(workers = self.workers, "metadata resolver started");

        // Wait for DHT routing tables to populate
        loop {
            if self.cancel.is_cancelled() {
                return;
            }
            let stats = self.engine.stats();
            if stats.total_routing_nodes >= 20 {
                tracing::info!(
                    nodes = stats.total_routing_nodes,
                    "routing table ready, starting resolver"
                );
                break;
            }
            tracing::debug!(
                nodes = stats.total_routing_nodes,
                "waiting for routing table..."
            );
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        }

        let mut last_stats = Instant::now();
        let _resolved_count = 0u64;
        let _failed_count = 0u64;
        let semaphore = Arc::new(tokio::sync::Semaphore::new(self.workers));

        loop {
            if self.cancel.is_cancelled() {
                break;
            }

            // Get unresolved hashes to work on
            let batch = self.get_unresolved_batch(self.workers * 2).await;

            if batch.is_empty() {
                tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                continue;
            }

            // Discover peers for the batch via DHT
            let hash_strings: Vec<String> = batch.to_vec();
            let discovered = self.engine.discover_peers(&hash_strings).await;

            // Also check cached peers
            for hash in &hash_strings {
                if !discovered.contains_key(hash) {
                    let cached = self.shared.get_peers(hash);
                    if !cached.is_empty() {
                        let addrs: Vec<SocketAddr> = cached
                            .iter()
                            .filter_map(|(ip, port)| format!("{ip}:{port}").parse().ok())
                            .collect();
                        if !addrs.is_empty() {
                            discovered.insert(hash.clone(), addrs);
                        }
                    }
                }
            }

            // Resolve each hash that has peers
            for entry in discovered.iter() {
                if self.cancel.is_cancelled() {
                    break;
                }
                let hash = entry.key().clone();
                let peers = entry.value().clone();
                if peers.is_empty() {
                    continue;
                }

                let permit = match semaphore.clone().acquire_owned().await {
                    Ok(p) => p,
                    Err(_) => break,
                };

                let pool = self.pool.clone();
                let timeout = self.timeout_secs;
                let threshold = self.save_files_threshold;
                let cancel = self.cancel.clone();

                tokio::spawn(async move {
                    let _permit = permit;
                    if cancel.is_cancelled() {
                        return;
                    }

                    // Increment resolve attempts
                    let _ = sqlx::query("UPDATE torrents SET resolve_attempts = resolve_attempts + 1 WHERE info_hash = $1")
                        .bind(&hash)
                        .execute(&pool)
                        .await;

                    match fetch_metadata(&hash, &peers, timeout).await {
                        Ok(meta) => {
                            if let Err(e) = process_resolved(&pool, &hash, &meta, threshold).await {
                                tracing::debug!(hash = %hash, error = %e, "failed to store metadata");
                            }
                        }
                        Err(e) => {
                            tracing::trace!(hash = %hash, error = %e, "metadata fetch failed");
                        }
                    }
                });
            }

            // Increment attempts for hashes with no peers
            for hash in &hash_strings {
                if !discovered.contains_key(hash) {
                    let _ = sqlx::query("UPDATE torrents SET resolve_attempts = resolve_attempts + 1 WHERE info_hash = $1")
                        .bind(hash)
                        .execute(&self.pool)
                        .await;
                }
            }

            // Stats
            if last_stats.elapsed().as_secs() >= 30 {
                let total: i64 = sqlx::query_scalar(
                    "SELECT COUNT(*) FROM torrents WHERE resolved_at IS NOT NULL",
                )
                .fetch_one(&self.pool)
                .await
                .unwrap_or(0);
                tracing::info!(total_resolved = total, "resolver stats");
                last_stats = Instant::now();
            }

            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }

        tracing::info!("metadata resolver stopped");
    }

    /// Get a batch of unresolved info_hashes from the DB.
    async fn get_unresolved_batch(&self, limit: usize) -> Vec<String> {
        // First check cached peers for fast path
        let cached_hashes: Vec<String> = self
            .shared
            .peer_cache
            .iter()
            .take(limit * 2)
            .map(|e| e.key().clone())
            .collect();

        if !cached_hashes.is_empty() {
            // Filter to unresolved only
            let rows = sqlx::query(
                "SELECT info_hash FROM torrents \
                 WHERE info_hash = ANY($1) \
                   AND resolved_at IS NULL \
                   AND resolve_attempts < 5 \
                   AND source != 'uploaded' \
                 ORDER BY priority DESC, observations DESC \
                 LIMIT $2",
            )
            .bind(&cached_hashes)
            .bind(limit as i64)
            .fetch_all(&self.pool)
            .await;

            if let Ok(rows) = rows {
                let mut result: Vec<String> = rows.iter().map(|r| r.get("info_hash")).collect();
                if result.len() >= limit {
                    return result;
                }

                // Fill remaining from DB
                let remaining = limit - result.len();
                if let Ok(more) = sqlx::query(
                    "SELECT info_hash FROM torrents \
                     WHERE resolved_at IS NULL \
                       AND resolve_attempts < 5 \
                       AND source != 'uploaded' \
                     ORDER BY priority DESC, \
                              CASE WHEN source IN ('announce', 'get_peers') THEN 0 ELSE 1 END, \
                              observations DESC, \
                              discovered_at DESC \
                     LIMIT $1",
                )
                .bind(remaining as i64)
                .fetch_all(&self.pool)
                .await
                {
                    for r in more {
                        let h: String = r.get("info_hash");
                        if !result.contains(&h) {
                            result.push(h);
                        }
                    }
                }
                return result;
            }
        }

        // Fallback: just query DB
        sqlx::query(
            "SELECT info_hash FROM torrents \
             WHERE resolved_at IS NULL \
               AND resolve_attempts < 5 \
               AND source != 'uploaded' \
             ORDER BY priority DESC, \
                      CASE WHEN source IN ('announce', 'get_peers') THEN 0 ELSE 1 END, \
                      observations DESC, \
                      discovered_at DESC \
             LIMIT $1",
        )
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default()
        .iter()
        .map(|r| r.get("info_hash"))
        .collect()
    }
}

/// Resolved metadata from a peer.
#[derive(Debug)]
struct ResolvedMeta {
    name: String,
    size: i64,
    files: Vec<FileEntry>,
    is_private: bool,
    piece_length: Option<i32>,
    piece_count: Option<i32>,
    seed_count: i32,
    peer_count: i32,
}

#[derive(Debug)]
struct FileEntry {
    path: String,
    size: i64,
}

/// Fetch metadata for an info_hash from peers.
///
/// This is a simplified implementation. The full BEP 9 metadata exchange
/// requires connecting to peers, performing a BitTorrent handshake with
/// extension protocol (BEP 10), negotiating ut_metadata, and downloading
/// metadata pieces. For Phase 3 MVP, we attempt to extract info from
/// the DHT response data (peer count) and mark as needing resolution.
///
/// A full implementation would use librtbit-peer-protocol's BEP 9 support
/// to connect to peers and download the metadata dictionary.
async fn fetch_metadata(
    info_hash: &str,
    peers: &[SocketAddr],
    _timeout_secs: u64,
) -> Result<ResolvedMeta, String> {
    // Phase 3 MVP: We don't yet implement the full BEP 9 peer connection.
    // This requires a TCP connection per peer with:
    // 1. BitTorrent handshake (protocol string + info_hash + peer_id)
    // 2. Extended handshake (BEP 10 - negotiate ut_metadata extension)
    // 3. Request metadata pieces (BEP 9 - ut_metadata request messages)
    // 4. Assemble and verify metadata (SHA1 of bencoded info == info_hash)
    //
    // The librtbit-peer-protocol crate has all the message types for this,
    // but the full TCP connection + handshake + piece assembly needs to be
    // wired up. For now, we return an error so torrents stay unresolved
    // and can be picked up when the full implementation is ready.
    //
    // TODO: Implement BEP 9 metadata fetching using:
    // - tokio::net::TcpStream for peer connections
    // - peer_binary_protocol::Message for handshake/extension messages
    // - SHA1 verification of assembled metadata
    Err(format!(
        "BEP 9 metadata fetch not yet implemented ({} peers available for {})",
        peers.len(),
        info_hash
    ))
}

/// Process resolved metadata — store in DB and run content pipeline.
async fn process_resolved(
    pool: &PgPool,
    info_hash: &str,
    meta: &ResolvedMeta,
    save_files_threshold: u32,
) -> Result<(), sqlx::Error> {
    // Parse and classify
    let parsed = indexarr_parser::parse(&meta.name);
    let file_infos: Vec<indexarr_classifier::FileInfo> = meta
        .files
        .iter()
        .map(|f| indexarr_classifier::FileInfo {
            path: f.path.clone(),
            size: f.size,
            extension: f.path.rsplit('.').next().map(|s| s.to_string()),
        })
        .collect();

    let classification = indexarr_classifier::classify(&parsed, &file_infos, &meta.name);
    let quality_score = indexarr_classifier::compute_quality_score(&parsed);

    // Update torrent record
    sqlx::query(
        "UPDATE torrents SET \
           name = $2, size = $3, resolved_at = NOW(), private = $4, \
           seed_count = GREATEST(seed_count, $5), peer_count = GREATEST(peer_count, $6), \
           no_peers = FALSE, priority = FALSE, \
           piece_length = $7, piece_count = $8 \
         WHERE info_hash = $1",
    )
    .bind(info_hash)
    .bind(&meta.name)
    .bind(meta.size)
    .bind(meta.is_private)
    .bind(meta.seed_count)
    .bind(meta.peer_count)
    .bind(meta.piece_length)
    .bind(meta.piece_count)
    .execute(pool)
    .await?;

    // Store files (if under threshold)
    if meta.files.len() <= save_files_threshold as usize {
        for file in &meta.files {
            let ext = file.path.rsplit('.').next().map(|e| e.to_lowercase());
            sqlx::query(
                "INSERT INTO torrent_files (info_hash, path, size, extension) VALUES ($1, $2, $3, $4)"
            )
            .bind(info_hash)
            .bind(&file.path)
            .bind(file.size)
            .bind(&ext)
            .execute(pool)
            .await?;
        }
    }

    // Store content classification
    sqlx::query(
        "INSERT INTO torrent_content (info_hash, content_type, title, year, season, episode, \
         \"group\", language, resolution, codec, video_source, modifier, is_3d, hdr, \
         audio_codec, audio_channels, edition, bit_depth, network, quality_score, \
         is_dubbed, is_complete, is_remastered, is_scene, is_proper, is_repack, \
         platform, has_subtitles, is_anime, music_format, classified_at, classifier_version) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, \
                 $17, $18, $19, $20, $21, $22, $23, $24, $25, $26, $27, $28, $29, $30, NOW(), '0.1.0') \
         ON CONFLICT (info_hash) DO UPDATE SET \
           content_type = EXCLUDED.content_type, title = EXCLUDED.title, year = EXCLUDED.year, \
           quality_score = EXCLUDED.quality_score, classified_at = NOW()"
    )
    .bind(info_hash)
    .bind(&classification.content_type)
    .bind(&parsed.title)
    .bind(parsed.year)
    .bind(parsed.season)
    .bind(parsed.episode)
    .bind(&parsed.group)
    .bind(parsed.languages.first().map(|s| s.as_str()))
    .bind(&parsed.resolution)
    .bind(&parsed.codec)
    .bind(&parsed.video_source)
    .bind(&parsed.modifier)
    .bind(parsed.is_3d)
    .bind(parsed.hdr.first().map(|s| s.as_str()))
    .bind(parsed.audio_codecs.first().map(|s| s.as_str()))
    .bind(&parsed.audio_channels)
    .bind(&parsed.edition)
    .bind(&parsed.bit_depth)
    .bind(&parsed.network)
    .bind(quality_score)
    .bind(parsed.is_dubbed)
    .bind(parsed.is_complete)
    .bind(parsed.is_remastered)
    .bind(parsed.is_scene)
    .bind(parsed.is_proper)
    .bind(parsed.is_repack)
    .bind(classification.platform.or(parsed.platform.clone()))
    .bind(classification.has_subtitles || parsed.has_subtitles)
    .bind(classification.is_anime)
    .bind(&classification.music_format)
    .execute(pool)
    .await?;

    // Store tags
    for tag in &classification.tags {
        let _ = sqlx::query(
            "INSERT INTO torrent_tags (info_hash, tag, source) VALUES ($1, $2, 'classifier') \
             ON CONFLICT (info_hash, tag) DO NOTHING",
        )
        .bind(info_hash)
        .bind(tag)
        .execute(pool)
        .await;
    }

    tracing::debug!(
        hash = %info_hash,
        name = %meta.name,
        content_type = %classification.content_type,
        quality = quality_score,
        "resolved torrent"
    );

    Ok(())
}
