pub mod health;
pub mod search;
pub mod stats;
pub mod queue;
pub mod torrents;
pub mod categories;
pub mod upload;
pub mod social;
pub mod identity;
pub mod system;
pub mod crud;

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
}
