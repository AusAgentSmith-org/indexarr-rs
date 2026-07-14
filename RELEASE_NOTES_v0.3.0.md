# Indexarr v0.3.0

## Summary

Indexarr 0.3.0 restores real BEP 51 crawling, prevents stale uploads from
starving newly discovered hashes, and makes resolver performance observable by
source and failure category. It also upgrades the Rust baseline and major
workspace dependencies, secures private-registry Docker builds, and makes
metadata persistence atomic.

## DHT crawling and issue #3

- The crawler now submits BEP 51 `sample_infohashes` queries and ingests the
  returned 20-byte samples as real torrent hashes.
- Random `get_peers` lookup targets are no longer mistaken for observations.
- Unresolved rows created by the former `get_peers` path are removed during
  migration because they were random lookup keys, not discovered torrents.
- BEP 51 responses are validated against their transaction IDs before samples
  are accepted.

This fixes the root cause of issue #3, where the crawler appeared active but
the index never gained resolvable DHT discoveries.

## Fair resolver scheduling

- Half of every resolver claim is reserved for never-attempted BEP 51 hashes.
- Priority uploads are capped at one quarter of a batch, preventing old upload
  retries from consuming every worker slot.
- The remaining quarter serves the general retry backlog; unused reservations
  flow to eligible non-upload work.
- Claims use row locking and a ten-minute in-flight lease. The normal
  exponential retry delay begins only after an attempt completes, so a slow
  peer race cannot be claimed by another resolver loop.
- Retry delays start at 30 seconds and cap at one day.

## Resolver observability

- Resolver attempts are retained for seven days with source, attempt number,
  outcome, candidate-peer count, duration, and the last error.
- Peer failures are classified as connection, missing BEP 10 support, timeout,
  or other protocol failure.
- `/api/v1/stats` and `/api/v1/dht/status` expose rolling per-source attempts,
  successes, failures, success rate, average duration, peer counts, and failure
  categories.
- Resolver logs include source, attempt, retry time, and the structured peer
  failure breakdown.

## Metadata integrity

- Locally fetched torrent metadata, files, classification, and tags are now
  committed in one PostgreSQL transaction.
- Extensible classifier fields use `TEXT` so a future label cannot invalidate
  an otherwise successful metadata fetch.
- Locally resolved rows left incomplete by older builds are safely requeued.
- File-extension detection now requires a real suffix and handles both Unix
  and Windows path separators.

## Toolchain and dependency refresh

- The supported toolchain and Woodpecker Rust images move to Rust 1.97.
- Major upgrades include SQLx 0.9, Reqwest 0.13, Rand 0.10, Ed25519 Dalek 3,
  SHA-2 0.11, Tower HTTP 0.7, and Quick XML 0.41.
- The latest compatible `librtbit` family releases are used.
- Tokio-XMPP remains on version 4 because version 6 requires redesigning the
  custom connector and stanza-handling path.
- Runtime and API version reporting now comes from package metadata instead of
  stale hard-coded `0.1.0` strings.

## Container and release security

- Private Forgejo Cargo credentials are supplied to Docker through BuildKit
  secrets and removed within the build step; they are not Docker arguments,
  environment values, or retained layers.
- Release containers carry OCI source, license, version, and revision labels.
- Docker builds use the Rust 1.97 builder and reproducible `npm ci` UI install.
- The UI dependency lockfile is refreshed and passes `npm audit` with no known
  vulnerabilities.
- Release publication builds immutable amd64 and arm64 candidates, smoke-tests
  the exact image, and only then advances versioned and `latest` tags in both
  Forgejo and GHCR.

## Validation

- Rust 1.97 formatting, all-target checking, strict Clippy, workspace tests,
  documentation tests, and the vendored-OpenSSL release path pass.
- Regression coverage verifies BEP 51 transaction matching and sample
  ingestion, fair lane allocation, retry timing, peer-failure classification,
  and file-extension handling.
- The production scheduler completed thousands of attempts with no overlapping
  intervals after the claim-lease change.

## Upgrade notes

- PostgreSQL schema updates run automatically at startup.
- No intentional REST, Torznab, sync, or configuration compatibility break is
  introduced.
- Operators using containers should pin `v0.3.0`. The `latest` tag is updated
  only after the tagged multi-architecture image passes verification.
- Existing data and container mounts remain compatible.

## Downloads

- Linux x86_64: `indexarr-v0.3.0-linux-x86_64.tar.gz`
- Linux aarch64: `indexarr-v0.3.0-linux-aarch64.tar.gz`
- Linux installer: `indexarr-v0.3.0-linux-install.sh`
- Windows x86_64 binary: `indexarr-v0.3.0-windows-x86_64.exe`
- Windows x86_64 installer: `indexarr-v0.3.0-windows-x86_64-setup.exe`
- Checksums: `SHA256SUMS-v0.3.0.txt`
- Docker: `ghcr.io/ausagentsmith-org/indexarr-rs:v0.3.0`

All downloadable files are attached to the public GitHub release.
