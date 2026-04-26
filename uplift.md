# Indexarr-rs uplift — peer & tracker discovery

Tracking the gap between what Python Indexarr does today and what indexarr-rs
needs to fully self-resolve metadata, harvest trackers, and surface real
seed/peer counts.

Status: Tier 1 complete. Tier 2 complete. Tier 3 complete (3.1–3.3 shipped;
3.4 out of scope). See bep-uplift.md Phase D execution log for details.

---

## Background — why this matters

`indexarr-rs` ships:
- A DHT crawler (`indexarr-dht::engine`) that discovers info_hashes via random
  `get_peers` queries.
- A resolver (`indexarr-dht::resolver`) intended to fetch torrent metadata
  via BEP 9 (`ut_metadata`).
- An announcer (`indexarr-announcer`) that scrapes HTTP trackers for seed/peer
  counts.
- A sync engine (`indexarr-sync`) that gossip-merges deltas with peers.

Three concrete gaps surfaced after the hertzde3 → bulk-import migration:

1. **31,705 imported entries have `seed_count = 0, peer_count = 0`**. They
   were imported without trackers (the import script + endpoint dropped the
   field), so the announcer has nothing to scrape — every "announce" is a
   no-op that just bumps `announced_at`.
2. **`fetch_metadata()` in `crates/indexarr-dht/src/resolver.rs:291` is a
   stub** that returns `Err("BEP 9 metadata fetch not yet implemented...")`.
   `total_resolved` from the resolver stays at 0 even though peers are
   discovered — the resolver finds them, then can't actually pull the
   .torrent metadata from them.
3. **The announcer is HTTP-only** (line 284: "Only HTTP trackers for now,
   UDP tracker BEP 15 scrape deferred"). Most of the public-tracker swarm
   is UDP, so even when trackers are present, half the time we can't scrape.

A subtle but important point: **even a fully-implemented BEP 9 won't recover
trackers**. BEP 9 (`ut_metadata`) transmits only the *info dict* — the part
of the .torrent file that gets SHA1-hashed to make the info_hash. The
`announce` / `announce-list` fields live in the **outer bencoded dict** and
are not transmitted. Trackers genuinely have to come from one of:

- Sync peers sharing them in deltas (Python's main path; broken for our bulk
  import because `/api/v1/import` doesn't accept the field).
- Users uploading .torrent files (the `/upload` route).
- External APIs / scrapers (Python has `dht/api_sources.py`).
- **BEP 28 (`lt_tex`, Tracker Exchange)** — peers gossip their tracker lists
  over the existing peer connection. This is the right long-term answer.
- A static "default tracker list" applied to trackerless hashes.

---

## Tier 1 — quick wins

Goal: stop bleeding new trackerless data and unblock the existing 31k entries.

### 1.1 Default tracker list

**Status**: already half-wired — `Settings.default_trackers` exists, is loaded
via `env_list("INDEXARR_DEFAULT_TRACKERS", ...)` in
`crates/indexarr-core/src/config.rs:159`, and `parse_trackers()` in the
announcer at `crates/indexarr-announcer/src/lib.rs:252-260` already returns
the defaults when the DB tracker array is null/empty. **What's missing**: the
default value list itself is empty, so trackerless torrents currently fall
through to a no-op scrape.

Action:
- Set a curated baseline in `Settings::from_env`'s call to `env_list("INDEXARR_DEFAULT_TRACKERS", &[...])`
  with a vetted public-tracker list (mix of UDP + HTTPS so it stays useful
  once Tier 2.1 lands). Initial picks:
  - `udp://tracker.opentrackr.org:1337/announce`
  - `udp://tracker.openbittorrent.com:80/announce`
  - `udp://exodus.desync.com:6969/announce`
  - `udp://open.tracker.cl:1337/announce`
  - `https://tracker.gbitt.info:443/announce`
  - `http://tracker.openbittorrent.com:80/announce`

No DB mutation needed — fallback is computed at scrape time.

### 1.2 `/api/v1/import` accepts the full record

**Status**: ✅ **Complete** (2026-04-26, commit `b2502ec`). `ImportItem` now
accepts `trackers`, `nfo`, `seed_count`, `peer_count`, `discovered_at` and
binds them in the INSERT.

Action:
- Extend `ImportItem` with optional fields:
  - `trackers: Option<Vec<String>>`
  - `nfo: Option<String>`
  - `seed_count: Option<i32>`, `peer_count: Option<i32>`
  - `discovered_at: Option<DateTime<Utc>>` (preserve original timestamp)
- Update the `INSERT INTO torrents (...)` to bind the new columns
  (`trackers` is JSONB, so serialize via `serde_json::to_value`).

### 1.3 Sync-merge tracker writeback

**Status**: confirmed working. `crates/indexarr-sync/src/merge.rs:190-206`
already pulls `record.get("trackers")` from the delta record and binds it
into the upsert. So when peers gossip their trackers via P2P sync, those
land in our DB.

Action: **none** — audited only. Document in this file so we don't go
re-implementing it.

---

## Tier 2 — moderate effort, ship same session

### 2.1 BEP 15 — UDP tracker scrape

**Status**: ✅ **Complete** (commit `a699787`).

**Spec**: <https://www.bittorrent.org/beps/bep_0015.html>

Action:
- Add `librtbit-tracker-comms = { version = "3", registry = "forgejo" }` to
  `indexarr-announcer/Cargo.toml`.
- Replace the "skip if not http" branch in `scrape_trackers()` with a
  match on URL scheme: HTTP path stays as-is, UDP path delegates to
  `tracker-comms`'s scrape API.
- Multi-hash batching (multiple info_hashes per scrape request) is
  supported by the spec and by the crate — significant efficiency win
  over per-hash HTTP scrape. Wire it up at the `poll_and_harvest` layer
  by grouping handles by tracker URL before scraping.

### 2.2 DHT peer-count refresher worker

**Status**: ✅ **Complete** (2026-04-26, commit `b2502ec`). `peer_refresher`
worker in `crates/indexarr-dht/src/peer_refresher.rs`. Included in `--all`.
Interval: `INDEXARR_PEER_REFRESH_INTERVAL` (default 300s); batch:
`INDEXARR_PEER_REFRESH_BATCH` (default 100).

New worker (~60 lines, lives in `indexarr-dht::peer_refresher` or similar):

1. Every `INDEXARR_PEER_REFRESH_INTERVAL` seconds (default 300):
2. Pick a batch of N hashes (default 100) from `torrents` ordered by
   `announced_at ASC NULLS FIRST`, scoped to those whose trackers list is
   empty OR whose `announced_at` is older than some threshold.
3. Call `engine.discover_peers(&batch)`.
4. For each `(hash, peers)`, run:
   ```sql
   UPDATE torrents
      SET peer_count = GREATEST(peer_count, $2),
          announced_at = NOW()
    WHERE info_hash = $1
   ```
   where `$2 = peers.len() as i32` (deduplicated by IP+port).

Caveats:
- DHT can't distinguish seeders from leechers — `seed_count` stays 0 unless
  a tracker scrape (Tier 2.1) or BEP 9 metadata fetch (Tier 3.1) populates it.
  Document this assumption in code.
- DHT lookup is bursty; batch size + interval should respect the DHT
  routing table's capacity. Start conservative (100 per 5min) and tune.

### 2.3 Wire it all together

- Add a new `--workers peer_refresher` selectable worker.
- Default `--all` includes it.
- Ensure the new worker shares the same `DhtEngine` instance the crawler
  uses (no separate DHT — it's already a heavy dependency).

---

## Tier 3 — leverage rustTorrent's `librtbit-*` crates

**Discovery (2026-04-25)**: rustTorrent (`~/Working/Active/apps/rustTorrent`)
already implements BEP 9 / 10 / 11 / 15 in production crates published to
the Forgejo registry. `indexarr-rs` already pulls `librtbit-dht` from the
same family, so the supply chain is established. This collapses Tier 3
from "1-2 days" to "a few hours of integration work".

Available off-the-shelf (`registry = "forgejo"`):

| BEP | Crate | Module | Status |
|---|---|---|---|
| 9 (`ut_metadata`) | `librtbit-peer-protocol = "4.3.0"` | `extended/ut_metadata.rs` | full message impl |
| 10 (extended handshake) | `librtbit-peer-protocol = "4.3.0"` | `extended/handshake.rs` | full |
| 11 (`ut_pex`) | `librtbit-peer-protocol = "4.3.0"` | `extended/ut_pex.rs` | full |
| 15 (UDP tracker) | `librtbit-tracker-comms = "3.0.0"` | `tracker_comms_udp.rs` | full connect/announce/scrape |
| 28 (`lt_tex`) | — | — | **not implemented anywhere** — must be new code |

There's also `librtbit::peer_info_reader` in rustTorrent which *looks like*
the orchestration layer for BEP 9 (TCP connect → handshake → drive piece
fetch). Verify whether it's directly usable or whether we need to vendor
the orchestration code.

The four items still need to ship together because they all share the
same BEP 10 extension handshake plumbing — but it's "wire the existing
crates together" work, not "implement the wire format from scratch".

### 3.1 BEP 9 — ut_metadata fetch (replace the stub)

**Status**: ✅ **Complete** (commit `c350bb6`).

**Spec**: <https://www.bittorrent.org/beps/bep_0009.html>

Approach: depend on `librtbit-peer-protocol` and either (a) call into
rustTorrent's `librtbit::peer_info_reader` if it can be reused as-is, or
(b) write a thin orchestrator that uses the message types from
`peer_binary_protocol::extended::{handshake, ut_metadata}` directly.

Sketch of the orchestrator path:
1. TCP connect (`tokio::net::TcpStream`) to a peer from
   `engine.discover_peers()`.
2. Send standard BitTorrent handshake (BEP 3) using
   `peer_binary_protocol`'s handshake type — set the BEP 10 extension bit
   in the reserved field.
3. Send BEP 10 extended handshake announcing `ut_metadata`, `ut_pex`, and
   `lt_tex` in our `m` dict.
4. Receive peer's extended handshake → pull their `ut_metadata` id and
   `metadata_size`.
5. Loop: build `UtMetadata::Request { piece }` messages and write via the
   extension channel; receive `UtMetadata::Data { piece, total_size, .. }`,
   accumulate.
6. SHA1-verify reassembled bytes against `info_hash`, bencode-decode the
   info dict.
7. Update `ResolvedMeta` (extend it to include the bit 21 `private` flag
   and full file list — already implemented in the struct).
8. Update `process_resolved()` to persist (already does most of this; the
   missing field is `nfo`, which we already have a column for).

Key: `peer_binary_protocol`'s `UtMetadata` enum already serializes/deserializes
the BEP 9 messages — no bencode work needed on our side beyond the final
info-dict parse.

### 3.2 BEP 28 — Tracker Exchange (`lt_tex`)

**Status**: ✅ **Receive-side complete** (2026-04-26, commit `b2502ec`).
We advertise `lt_tex` in every BEP 10 handshake and harvest incoming tracker
lists into `torrents.trackers`. Send-side (giving peers our tracker list)
deferred — requires the peer's lt_tex ID from their handshake, which is
currently dropped by `PeerExtendedMessageIds` deserialization.

**Spec**: <https://www.bittorrent.org/beps/bep_0028.html>

After 3.1 is wired:
1. In our extended handshake's `m` dict, advertise `lt_tex` with a local
   extension id.
2. When the peer's extended handshake includes their `lt_tex` id, send
   them a `lt_tex` message (bencoded dict: `{ "added": [<tracker urls>] }`)
   with our known tracker list.
3. When we receive a peer's `lt_tex` message, parse the `added` list,
   merge into `torrents.trackers` for that info_hash (de-dupe).

This is the **right long-term answer for tracker harvesting** — peers
gradually converge on the union of all known trackers per torrent, no
static fallback lists needed.

Implementation will need to add a small message type module mirroring the
shape of `extended/ut_pex.rs` — bencode struct with `added: Vec<String>`
and (optionally) `dropped: Vec<String>`.

### 3.3 BEP 11 — Peer Exchange (PEX, `ut_pex`)

**Status**: ✅ **Complete** (commit `7ae439a`).

**Spec**: <https://www.bittorrent.org/beps/bep_0011.html>
- Advertise `ut_pex` in our extended handshake's `m` dict.
- On receipt of a `ut_pex` message, decode the `added` / `added.f` peer
  lists and feed them into the DHT shared peer cache (the type
  `DhtSharedState` already supports this via `push_hash` / peer-cache).

### 3.4 BEP 12 — multi-tier `announce-list`

**Spec**: <https://www.bittorrent.org/beps/bep_0012.html>

Only relevant if/when we acquire the **outer** .torrent dict (e.g. by
fetching the .torrent file from an external HTTP source — torrent CDNs).
BEP 9 alone does NOT give us this. So 3.4 is contingent on a future feature
that fetches outer .torrent files. Out of scope for the current sprint;
documented here for completeness.

When implemented:
- Parse `announce-list: [[url1a, url1b], [url2a, url2b], ...]` (BEP 12 tier
  format) instead of just the single `announce`.
- Honour tier semantics in the announcer: shuffle within a tier, fall
  through to next tier on failure.

---

## Sequencing recommendation

```
Tier 1 ────────────────────────────────────► Tier 2 ────────► Tier 3
1.1 default trackers (config)                2.1 BEP 15        3.1 BEP 9   (use librtbit-peer-protocol)
1.2 import accepts trackers                  2.2 DHT refresh   3.2 BEP 28  ◄── tracker harvest (NEW code)
1.3 sync-merge audit (done)                  2.3 wire-up       3.3 BEP 11  (free, just enable in handshake)
                                                               3.4 BEP 12 (only after
                                                                   external .torrent fetch)
```

After Tier 2 lands the announcer is meaningfully useful — UDP scrape works,
defaults cover trackerless hashes, DHT keeps peer counts fresh. After
Tier 3 lands the resolver actually resolves and trackers organically
converge across the swarm.

Tier 3 effort estimate **dropped from ~2 days to a few hours** thanks to
the rustTorrent `librtbit-*` crates that already implement BEPs 9 / 10 /
11 / 15. Only BEP 28 (lt_tex) is genuinely new code, and it's small — one
extension message type slotted into the BEP 10 framework.

Anything not delivered by Tier 2 + Tier 3 (e.g. BEP 12 outer-dict tier
handling, scrape multitracker hint extension) is icing.

---

## Cross-repo dependencies

The Tier 3 work introduces dependencies on rustTorrent-published crates
in our Forgejo cargo registry:

```toml
# indexarr-dht/Cargo.toml (already there)
dht = { version = "0.1.1", package = "librtbit-dht" }

# new
peer-protocol = { version = "4.3", package = "librtbit-peer-protocol", registry = "forgejo" }

# indexarr-announcer/Cargo.toml (new for Tier 2.1)
tracker-comms = { version = "3", package = "librtbit-tracker-comms", registry = "forgejo" }
```

Per the workspace `CLAUDE.md` shared-libs rules: any breaking change to
these crates from rustTorrent's side will affect indexarr-rs, so we should
pin versions explicitly and bump deliberately rather than tracking
`latest`. Verify CI strips local `[patch]` overrides as documented.
