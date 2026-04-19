# Indexarr (Rust)

Indexarr — decentralized torrent indexing with DHT crawling, content classification, gossip-based P2P sync, and contributor identity.

## Architecture

10-crate Rust workspace. Single binary (`indexarr`) runs selectable workers via `--workers` flag. All workers share a PostgreSQL database through sqlx async pool.

### Tech Stack
- **Rust 2024 edition** — Axum 0.8, SQLx 0.8 (async PostgreSQL), Tokio 1.x
- **PostgreSQL 17** (production)
- **librtbit-dht** — DHT protocol (BEP 5), peer discovery
- **librtbit-peer-protocol** — BitTorrent wire protocol (BEP 9/10)
- **Vue 3 + Vite + TypeScript + Pinia** — Frontend SPA
- **Docker** — Multi-stage build (Node 22 for UI, Rust for backend, Debian slim runtime)

## Development

```bash
# Docker (recommended)
docker compose up -d

# Local development
cargo build
INDEXARR_DB_URL=postgres://indexarr:indexarr@localhost:5432/indexarr \
  cargo run -- --workers http_server

# Tests
cargo test --workspace

# All workers
cargo run -- --all
```

## Workers

| Worker | What it does |
|--------|-------------|
| `http_server` | Axum HTTP — REST API, Torznab XML, Vue SPA |
| `dht_crawler` | N librtbit-dht instances crawling for info_hashes |
| `resolver` | BEP 9 metadata fetching + content pipeline |
| `announcer` | HTTP tracker scrape for seed/peer validation |
| `sync` | 3-loop gossip protocol (export, discovery, gossip) |

## Crate Map

```
indexarr-core        Config, models, DB schema, errors
indexarr-identity    Ed25519 keypair, contributor ID, ban list
indexarr-parser      Torrent name parser (regex-based)
indexarr-classifier  Content type detection, quality scoring
indexarr-search      PostgreSQL FTS + 16 faceted filters
indexarr-web         Axum server, ~35 REST endpoints, Torznab XML
indexarr-tmdb        TMDB client with rate limiter + circuit breaker
indexarr-dht         DHT engine, hash ingest, metadata resolver
indexarr-announcer   Rolling pool tracker scraper
indexarr-sync        Gossip protocol, delta export/merge, peer discovery, epoch
```

## Environment Variables

All prefixed with `INDEXARR_`. See `crates/indexarr-core/src/config.rs` for the full list.

## Docker

| File | Purpose |
|------|---------|
| `docker-compose.yml` | All workers (DHT + resolver + announcer + sync + HTTP) |
| `docker-compose.sync.yml` | Web UI + sync only (no crawling) |

## Tests

```bash
cargo test --workspace    # 10 tests (parser + classifier)
```
