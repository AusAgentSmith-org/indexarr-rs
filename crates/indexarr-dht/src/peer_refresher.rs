use std::sync::Arc;

use sqlx::{PgPool, Row};
use tokio_util::sync::CancellationToken;

use crate::engine::DhtEngine;

/// Periodically picks stale torrents from the DB and queries the DHT for live
/// peers, writing the discovered peer count back. Refreshes hashes that have
/// empty tracker lists first (they can't be reached via announcer) and then
/// those with the oldest `announced_at`.
pub async fn run_peer_refresher(
    pool: PgPool,
    engine: Arc<DhtEngine>,
    cancel: CancellationToken,
    batch_size: usize,
    interval_secs: u64,
) {
    tracing::info!(
        batch = batch_size,
        interval = interval_secs,
        "peer refresher started"
    );

    // Give the DHT routing table time to populate before the first scan.
    tokio::time::sleep(std::time::Duration::from_secs(60)).await;

    loop {
        if cancel.is_cancelled() {
            break;
        }

        let batch: Vec<String> = sqlx::query(
            "SELECT info_hash FROM torrents \
             WHERE resolved_at IS NOT NULL \
             ORDER BY \
               CASE WHEN trackers IS NULL OR jsonb_array_length(trackers) = 0 THEN 0 ELSE 1 END ASC, \
               COALESCE(announced_at, '1970-01-01'::timestamptz) ASC \
             LIMIT $1",
        )
        .bind(batch_size as i64)
        .fetch_all(&pool)
        .await
        .unwrap_or_default()
        .iter()
        .map(|r| r.get("info_hash"))
        .collect();

        if batch.is_empty() {
            tokio::time::sleep(std::time::Duration::from_secs(interval_secs)).await;
            continue;
        }

        tracing::debug!(count = batch.len(), "peer refresher: querying DHT");
        let discovered = engine.discover_peers(&batch).await;

        let mut updated = 0usize;
        for entry in discovered.iter() {
            let count = entry.value().len() as i32;
            if count == 0 {
                continue;
            }
            let _ = sqlx::query(
                "UPDATE torrents \
                 SET peer_count = GREATEST(peer_count, $2), \
                     announced_at = NOW() \
                 WHERE info_hash = $1",
            )
            .bind(entry.key().as_str())
            .bind(count)
            .execute(&pool)
            .await;
            updated += 1;
        }

        tracing::info!(
            batch = batch.len(),
            found = discovered.len(),
            updated,
            "peer refresher: cycle complete"
        );

        tokio::time::sleep(std::time::Duration::from_secs(interval_secs)).await;
    }

    tracing::info!("peer refresher stopped");
}
