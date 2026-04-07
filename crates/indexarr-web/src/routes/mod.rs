pub mod categories;
pub mod crud;
pub mod health;
pub mod identity;
pub mod queue;
pub mod search;
pub mod social;
pub mod stats;
pub mod sync;
pub mod system;
pub mod torrents;
pub mod upload;

use std::sync::Arc;

use axum::Router;

use crate::state::AppState;

pub fn api_router(_state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .merge(search::router())
        .merge(stats::router())
        .merge(queue::router())
        .merge(torrents::router())
        .merge(categories::router())
        .merge(upload::router())
        .merge(social::router())
        .merge(identity::router())
        .merge(system::router())
        .merge(crud::router())
        .merge(sync::router())
}
