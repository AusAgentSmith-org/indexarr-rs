pub mod log_capture;
pub mod routes;
pub mod state;
pub mod torznab;

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use axum::Router;
use axum::extract::Request;
use axum::http::StatusCode;
use axum::middleware::{self, Next};
use axum::response::{Html, IntoResponse, Json, Response};
use tokio_util::sync::CancellationToken;
use tower_http::cors::{Any, CorsLayer};

use state::AppState;

/// Build the full Axum application router.
pub fn build_router(state: Arc<AppState>) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let api_routes = routes::api_router(state.clone());
    let torznab_routes = torznab::router(state.clone());

    let app = Router::new()
        .nest("/api/v1", api_routes)
        .nest("/api/torznab", torznab_routes)
        .route("/health", axum::routing::get(routes::health::health))
        .layer(cors)
        .layer(middleware::from_fn_with_state(
            state.clone(),
            startup_splash_middleware,
        ))
        .with_state(state.clone());

    // SPA fallback — serve static files and index.html for Vue Router
    add_spa_fallback(app, state)
}

fn add_spa_fallback(app: Router, _state: Arc<AppState>) -> Router {
    let ui_dist = find_ui_dist();
    if let Some(dist_path) = ui_dist {
        tracing::info!(path = %dist_path.display(), "serving Vue SPA from ui/dist");
        let serve_dir = tower_http::services::ServeDir::new(&dist_path)
            .fallback(tower_http::services::ServeFile::new(dist_path.join("index.html")));
        app.fallback_service(serve_dir)
    } else {
        tracing::warn!("ui/dist not found — SPA not available");
        app
    }
}

fn find_ui_dist() -> Option<PathBuf> {
    // Check relative to binary
    let candidates = [
        PathBuf::from("ui/dist"),
        PathBuf::from("/app/ui/dist"),
    ];
    for p in &candidates {
        if p.is_dir() {
            return Some(p.clone());
        }
    }
    None
}

async fn startup_splash_middleware(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
    request: Request,
    next: Next,
) -> Response {
    if !state.is_ready() {
        let path = request.uri().path();
        // Let health checks and identity endpoints through
        if path.starts_with("/health") || path.starts_with("/api/v1/identity/") {
            return next.run(request).await;
        }
        // API calls get 503
        if path.starts_with("/api/") || path.starts_with("/graphql") {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({
                    "detail": "Service starting up",
                    "ready": false,
                })),
            )
                .into_response();
        }
        // Browser requests get splash page
        return (StatusCode::SERVICE_UNAVAILABLE, Html(STARTUP_HTML)).into_response();
    }
    next.run(request).await
}

/// Run the HTTP server until the cancellation token is triggered.
pub async fn run_server(
    state: Arc<AppState>,
    host: &str,
    port: u16,
    cancel: CancellationToken,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let app = build_router(state);
    let addr: SocketAddr = format!("{host}:{port}").parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!(%addr, "HTTP server listening");

    axum::serve(listener, app)
        .with_graceful_shutdown(async move { cancel.cancelled().await })
        .await?;

    Ok(())
}

const STARTUP_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Indexarr — Starting Up</title>
<meta http-equiv="refresh" content="3">
<style>
  * { margin: 0; padding: 0; box-sizing: border-box; }
  body {
    min-height: 100vh; display: flex; align-items: center; justify-content: center;
    background: #0a0a1a; color: #e8e8e8;
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
  }
  .wrap { text-align: center; }
  .logo {
    width: 64px; height: 64px; color: #e94560; margin-bottom: 24px;
    animation: pulse 2s ease-in-out infinite;
  }
  @keyframes pulse { 0%,100% { opacity:.7; transform:scale(1); } 50% { opacity:1; transform:scale(1.05); } }
  h1 { font-size: 1.5rem; font-weight: 700; margin-bottom: 8px; }
  p { color: #8888aa; font-size: .9rem; margin-bottom: 24px; }
  .dots span {
    display: inline-block; width: 8px; height: 8px; border-radius: 50%;
    background: #e94560; margin: 0 4px;
    animation: bounce 1.4s ease-in-out infinite;
  }
  .dots span:nth-child(2) { animation-delay: .2s; }
  .dots span:nth-child(3) { animation-delay: .4s; }
  @keyframes bounce { 0%,80%,100% { opacity:.3; transform:scale(.8); } 40% { opacity:1; transform:scale(1.2); } }
</style>
</head>
<body>
<div class="wrap">
  <svg class="logo" viewBox="0 0 48 48" fill="none" stroke="currentColor" stroke-width="1.5">
    <circle cx="24" cy="24" r="20"/><path d="M24 4a30.6 30.6 0 0 1 8 20 30.6 30.6 0 0 1-8 20 30.6 30.6 0 0 1-8-20A30.6 30.6 0 0 1 24 4z"/>
    <path d="M4 24h40"/>
  </svg>
  <h1>Indexarr is starting up</h1>
  <p>Initializing workers and database&hellip;</p>
  <div class="dots"><span></span><span></span><span></span></div>
</div>
</body>
</html>"#;
