pub mod engine;
pub mod ingest;
pub mod resolver;

use std::collections::VecDeque;
use std::sync::Arc;

use dashmap::DashMap;
use parking_lot::Mutex;
use tokio_util::sync::CancellationToken;

/// Discovered info_hash with optional peer info and source.
#[derive(Debug, Clone)]
pub struct DiscoveredHash {
    pub info_hash: String,
    pub peer_ip: Option<String>,
    pub peer_port: Option<u16>,
    pub source: String,
}

/// Shared state between DHT engine, resolver, and ingest workers.
pub struct DhtSharedState {
    /// Queue of discovered info_hashes waiting for DB insertion.
    pub hash_queue: Mutex<VecDeque<DiscoveredHash>>,
    /// Peer cache: info_hash → list of (ip, port) peers.
    pub peer_cache: DashMap<String, VecDeque<(String, u16)>>,
    /// Maximum peer cache size (evict oldest when exceeded).
    pub max_cache_size: usize,
    /// Maximum hash queue depth.
    pub max_queue_size: usize,
}

impl DhtSharedState {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            hash_queue: Mutex::new(VecDeque::with_capacity(10_000)),
            peer_cache: DashMap::new(),
            max_cache_size: 200_000,
            max_queue_size: 100_000,
        })
    }

    /// Push a discovered hash into the queue.
    pub fn push_hash(&self, hash: DiscoveredHash) {
        // Cache peer if available
        if let (Some(ip), Some(port)) = (&hash.peer_ip, hash.peer_port) {
            let mut peers = self
                .peer_cache
                .entry(hash.info_hash.clone())
                .or_insert_with(|| VecDeque::with_capacity(20));
            if peers.len() < 20 {
                peers.push_back((ip.clone(), port));
            }
        }

        let mut queue = self.hash_queue.lock();
        if queue.len() < self.max_queue_size {
            queue.push_back(hash);
        }
    }

    /// Drain up to `max` hashes from the queue.
    pub fn drain_hashes(&self, max: usize) -> Vec<DiscoveredHash> {
        let mut queue = self.hash_queue.lock();
        let count = max.min(queue.len());
        queue.drain(..count).collect()
    }

    /// Get cached peers for an info_hash.
    pub fn get_peers(&self, info_hash: &str) -> Vec<(String, u16)> {
        self.peer_cache
            .get(info_hash)
            .map(|p| p.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Evict oldest entries if cache is too large.
    pub fn evict_if_needed(&self) {
        if self.peer_cache.len() > self.max_cache_size {
            let to_remove = self.max_cache_size / 4;
            let keys: Vec<String> = self
                .peer_cache
                .iter()
                .take(to_remove)
                .map(|e| e.key().clone())
                .collect();
            for key in keys {
                self.peer_cache.remove(&key);
            }
            tracing::debug!(
                removed = to_remove,
                remaining = self.peer_cache.len(),
                "peer cache eviction"
            );
        }
    }
}

impl Default for DhtSharedState {
    fn default() -> Self {
        Self {
            hash_queue: Mutex::new(VecDeque::with_capacity(10_000)),
            peer_cache: DashMap::new(),
            max_cache_size: 200_000,
            max_queue_size: 100_000,
        }
    }
}
