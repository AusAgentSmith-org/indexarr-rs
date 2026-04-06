
use clap::Parser;
use tokio_util::sync::CancellationToken;

use indexarr_core::config::{DbBackend, Settings};
use indexarr_core::db;
use indexarr_identity::ContributorIdentity;
use indexarr_web::state::AppState;

#[derive(Parser)]
#[command(name = "indexarr", about = "Decentralized torrent indexing", version = "0.1.0")]
struct Cli {
    /// Run all workers (http_server, dht_crawler, resolver, announcer, sync)
    #[arg(long)]
    all: bool,

    /// Comma-separated list of workers to run
    #[arg(long)]
    workers: Option<String>,

    /// HTTP listen address
    #[arg(long)]
    host: Option<String>,

    /// HTTP listen port
    #[arg(long)]
    port: Option<u16>,

    /// Enable debug mode
    #[arg(long)]
    debug: bool,

    /// Database backend (postgresql or sqlite)
    #[arg(long, value_enum)]
    db_backend: Option<DbBackend>,

    /// Database URL
    #[arg(long)]
    db_url: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load .env if present
    let _ = dotenvy::dotenv();

    let cli = Cli::parse();

    // Load settings from env, then apply CLI overrides
    let mut settings = Settings::from_env();
    if let Some(host) = cli.host {
        settings.host = host;
    }
    if let Some(port) = cli.port {
        settings.port = port;
    }
    if cli.debug {
        settings.debug = true;
    }
    if let Some(backend) = cli.db_backend {
        settings.db_backend = backend;
    }
    if let Some(url) = cli.db_url {
        settings.db_url = url;
    }

    // Set up tracing
    let filter = if settings.debug { "debug" } else { "info" };
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(filter)),
        )
        .init();

    // Initialize contributor identity
    let mut identity = ContributorIdentity::new(&settings.data_dir);

    if !settings.contributor_recovery_key.is_empty() {
        match identity.restore_from_recovery_key(&settings.contributor_recovery_key) {
            Ok(()) => {
                identity.acknowledge_onboarding();
                tracing::info!(
                    id = identity.contributor_id().unwrap_or("?"),
                    "restored identity from env"
                );
            }
            Err(e) => {
                tracing::warn!(error = %e, "failed to restore identity from env");
                let (is_new, recovery_key) = identity.load_or_generate()?;
                log_identity(&identity, is_new, recovery_key.as_deref());
            }
        }
    } else {
        let (is_new, recovery_key) = identity.load_or_generate()?;
        log_identity(&identity, is_new, recovery_key.as_deref());
    }

    // Determine workers
    let workers: Vec<String> = if cli.all {
        vec![
            "http_server", "dht_crawler", "resolver", "announcer", "sync",
        ]
        .into_iter()
        .map(String::from)
        .collect()
    } else if let Some(ref w) = cli.workers {
        w.split(',').map(|s| s.trim().to_string()).collect()
    } else {
        settings.workers.clone()
    };

    tracing::info!(
        workers = workers.join(", "),
        "Indexarr v0.1.0 — starting workers"
    );

    // Initialize database
    let pool = db::init_db(&settings).await?;
    tracing::info!("database initialized");

    // Build shared state
    let host = settings.host.clone();
    let port = settings.port;
    let state = AppState::new(pool, settings, identity);

    // Cancellation token for graceful shutdown
    let cancel = CancellationToken::new();

    // Handle SIGINT/SIGTERM
    let cancel_signal = cancel.clone();
    tokio::spawn(async move {
        let ctrl_c = tokio::signal::ctrl_c();
        let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to listen for SIGTERM");

        tokio::select! {
            _ = ctrl_c => {}
            _ = sigterm.recv() => {}
        }
        tracing::info!("shutdown signal received");
        cancel_signal.cancel();
    });

    // Mark app ready once DB is confirmed reachable
    let state_ready = state.clone();
    tokio::spawn(async move {
        // DB is already connected at this point, mark ready
        state_ready.set_ready();
        tracing::info!("all systems ready");
    });

    // Start workers
    let mut handles = Vec::new();

    if workers.iter().any(|w| w == "http_server") {
        let state = state.clone();
        let cancel = cancel.clone();
        let host = host.clone();
        handles.push(tokio::spawn(async move {
            if let Err(e) = indexarr_web::run_server(state, &host, port, cancel).await {
                tracing::error!(error = %e, "HTTP server error");
            }
        }));
    }

    // DHT crawler + resolver
    let needs_dht = workers.iter().any(|w| w == "dht_crawler" || w == "resolver");
    let _dht_engine = if needs_dht {
        let dht_shared = indexarr_dht::DhtSharedState::new();
        let dht_instances = state.settings.dht_instances;
        let dht_base_port = state.settings.dht_base_port;

        match indexarr_dht::engine::DhtEngine::new(
            dht_instances,
            dht_base_port,
            dht_shared.clone(),
            cancel.clone(),
        ).await {
            Ok(engine) => {
                let engine = std::sync::Arc::new(engine);

                // Start hash ingest worker
                let ingest_pool = state.pool.clone();
                let ingest_shared = dht_shared.clone();
                let ingest_cancel = cancel.clone();
                handles.push(tokio::spawn(async move {
                    indexarr_dht::ingest::run_hash_ingest(ingest_pool, ingest_shared, ingest_cancel).await;
                }));

                // Start crawler
                if workers.iter().any(|w| w == "dht_crawler") {
                    let crawler_engine = engine.clone();
                    handles.push(tokio::spawn(async move {
                        crawler_engine.run_crawler().await;
                    }));
                }

                // Start resolver
                if workers.iter().any(|w| w == "resolver") {
                    let resolver = indexarr_dht::resolver::MetadataResolver::new(
                        state.pool.clone(),
                        dht_shared.clone(),
                        engine.clone(),
                        state.settings.resolve_workers as usize,
                        state.settings.resolve_timeout as u64,
                        state.settings.save_files_threshold,
                        cancel.clone(),
                    );
                    handles.push(tokio::spawn(async move {
                        resolver.run().await;
                    }));
                }

                Some(engine)
            }
            Err(e) => {
                tracing::error!(error = %e, "failed to start DHT engine");
                None
            }
        }
    } else {
        None
    };

    // Phase 4+: Announcer
    // Phase 5+: Sync
    for w in &workers {
        match w.as_str() {
            "http_server" | "dht_crawler" | "resolver" => {} // already handled
            "announcer" => tracing::warn!("announcer worker not yet implemented in Rust"),
            "sync" => tracing::warn!("sync worker not yet implemented in Rust"),
            other => tracing::warn!(worker = other, "unknown worker"),
        }
    }

    if handles.is_empty() {
        tracing::error!("no workers to run — use --all or --workers <list>");
        return Ok(());
    }

    // Wait for all workers
    cancel.cancelled().await;
    tracing::info!("shutting down...");

    for handle in handles {
        let _ = handle.await;
    }

    tracing::info!("shutdown complete");
    Ok(())
}

fn log_identity(identity: &ContributorIdentity, is_new: bool, recovery_key: Option<&str>) {
    if is_new {
        tracing::info!(
            id = identity.contributor_id().unwrap_or("?"),
            "new contributor identity generated"
        );
        if let Some(key) = recovery_key {
            tracing::info!(recovery_key = key, "save your recovery key!");
        }
    } else {
        tracing::info!(
            id = identity.contributor_id().unwrap_or("?"),
            "loaded contributor identity"
        );
    }
}
