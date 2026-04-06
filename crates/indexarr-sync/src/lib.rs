pub mod delta;
pub mod discovery;
pub mod epoch;
pub mod manager;
pub mod merge;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Activity log entry for sync dashboard.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncActivity {
    pub timestamp: DateTime<Utc>,
    pub event: String,
    pub message: String,
    pub peer_id: Option<String>,
}

/// Sync dashboard status exposed via API.
#[derive(Debug, Clone, Default, Serialize)]
pub struct SyncDashboard {
    pub enabled: bool,
    pub export_sequence: i64,
    pub peer_count: usize,
    pub healthy_peers: usize,
    pub last_export: Option<String>,
    pub last_gossip: Option<String>,
    pub epoch: i32,
    pub activity: Vec<SyncActivity>,
}
