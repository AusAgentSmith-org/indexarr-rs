use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

use indexarr_core::config::Settings;
use indexarr_identity::ContributorIdentity;
use parking_lot::RwLock;
use sqlx::PgPool;

/// Shared application state, accessible from all Axum handlers.
pub struct AppState {
    pub pool: PgPool,
    pub settings: Settings,
    pub identity: RwLock<ContributorIdentity>,
    pub started_at: Instant,
    ready: AtomicBool,
}

impl AppState {
    pub fn new(pool: PgPool, settings: Settings, identity: ContributorIdentity) -> Arc<Self> {
        Arc::new(Self {
            pool,
            settings,
            identity: RwLock::new(identity),
            started_at: Instant::now(),
            ready: AtomicBool::new(false),
        })
    }

    pub fn is_ready(&self) -> bool {
        self.ready.load(Ordering::Relaxed)
    }

    pub fn set_ready(&self) {
        self.ready.store(true, Ordering::Relaxed);
    }

    /// Check if the Torznab API key matches (empty key = no auth).
    pub fn check_api_key(&self, provided: &str) -> bool {
        let key = &self.settings.torznab_api_key;
        if key.is_empty() {
            return true;
        }
        key == provided
    }
}
