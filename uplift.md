# Indexarr-rs uplift — peer & tracker discovery

Tracking the gap between what Python Indexarr does today and what indexarr-rs
needs to fully self-resolve metadata, harvest trackers, and surface real
seed/peer counts.

Status: parts of Tier 1 are already wired (config defaults via
`INDEXARR_DEFAULT_TRACKERS`, sync-merge already persists `trackers`); Tier 2
is the next sprint; Tier 3 is the major follow-up.

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

**Status**: `crates/indexarr-web/src/routes/crud.rs:84-93`. `ImportItem`
currently only has `info_hash`, `name`, `size`, `source`, `files`. Drops
trackers, nfo, seed/peer counts on the floor.

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

**Spec**: <https://www.bittorrent.org/beps/bep_0015.html>

The announcer's `scrape_trackers()` at
`crates/indexarr-announcer/src/lib.rs:270-307` filters out `udp://` entries
("BEP 15 scrape deferred"). Most of the public swarm is UDP-only.

Implementation (~120 lines new code):

1. Parse `udp://host:port/announce` → resolve to `SocketAddr`.
2. **Connect handshake** (action 0):
   - Send 16-byte payload: `[protocol_id (8) = 0x41727101980, action (4) = 0, transaction_id (4) = random]`
   - Receive 16-byte response: `[action (4) = 0, transaction_id (4), connection_id (8)]`
   - Verify `transaction_id` matches.
3. **Scrape request** (action 2):
   - Send: `[connection_id (8), action (4) = 2, transaction_id (4) = random, info_hash (20) per hash, ...up to 70]`
   - Receive: `[action (4) = 2, transaction_id (4), (seeders (4), completed (4), leechers (4)) per info_hash]`
4. Use `tokio::net::UdpSocket` with a 5-second timeout per request.
5. Cache the `connection_id` for reuse within a 60-second window per tracker
   to amortize the handshake.

Multi-hash batching (multiple info_hashes per scrape) is permitted by the
spec and means one UDP round-trip can resolve many torrents — significant
efficiency win over per-hash HTTP scrape.

### 2.2 DHT peer-count refresher worker

The DHT engine already has `discover_peers(&[hash]) -> DashMap<hash, Vec<SocketAddr>>`
in `crates/indexarr-dht/src/engine.rs:163`. It works. Nothing currently
queues stale hashes through it for re-validation.

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

## Tier 3 — defer (full peer-protocol implementation)

These four items must ship **as one coherent piece** because they all build
on the same peer wire protocol and BEP 10 extension framework. Estimated
1-2 focused engineering days.

### 3.1 BEP 9 — ut_metadata fetch (replace the stub)

**Spec**: <https://www.bittorrent.org/beps/bep_0009.html>

`crates/indexarr-dht/src/resolver.rs:291-317` is currently a stub. Real
implementation:

1. **TCP connect** to a peer from the discovered list.
2. **BitTorrent handshake** (BEP 3): 68 bytes:
   `[protocol_string_len (1) = 19, protocol_string (19) = "BitTorrent protocol", reserved (8 — set bit 20 for ext support), info_hash (20), peer_id (20)]`.
3. **BEP 10 extended handshake** message: bencoded dict with `m: {"ut_metadata": <local_id>}` and `metadata_size: 0` (we don't know yet).
4. Receive peer's extended handshake; pull `ut_metadata` ID + `metadata_size` from their `m` dict.
5. Loop: send `request` message for each piece (`piece` index, 16 KiB chunks):
   `{"msg_type": 0, "piece": <i>}` over the extension channel.
6. Receive `data` messages: `{"msg_type": 1, "piece": <i>, "total_size": <bytes>}` followed by raw piece data.
7. Assemble pieces, SHA1-verify against `info_hash`, bencode-decode the info dict.
8. Pull name / size / files / piece info / private flag.
9. Update `process_resolved()` to actually persist these.

Add `librtbit-peer-protocol` as a workspace dep — it provides the message
types. The TCP layer is plain `tokio::net::TcpStream`.

### 3.2 BEP 28 — Tracker Exchange (`lt_tex`)

**Spec**: <https://www.bittorrent.org/beps/bep_0028.html>

This is what gives us **per-torrent trackers** organically, without static
default lists.

Once BEP 10 (3.1) is wired, advertise `lt_tex` in our extended handshake's
`m` dict. Peers who support it will send us a message containing their
tracker list for this torrent. Persist into `torrents.trackers`.

Sender side: when a peer sends us a `lt_tex` request, respond with our
own tracker list (after de-duping) so the swarm gradually converges on the
union of all known trackers.

### 3.3 BEP 11 — Peer Exchange (PEX, `ut_pex`)

**Spec**: <https://www.bittorrent.org/beps/bep_0011.html>

Less critical than 3.2 (DHT already gives us peers), but cheap once 3.1 is
in. Advertise `ut_pex` in extended handshake; peers send us `added` /
`added.f` lists of peers they've recently connected to. Feed those into the
DHT shared peer cache.

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
1.1 default trackers (config)                2.1 BEP 15        3.1 BEP 9
1.2 import accepts trackers                  2.2 DHT refresh   3.2 BEP 28 ◄── tracker harvest
1.3 sync-merge audit (done)                  2.3 wire-up       3.3 BEP 11
                                                               3.4 BEP 12 (only after
                                                                   external .torrent fetch)
```

After Tier 2 lands the announcer is meaningfully useful — UDP scrape works,
defaults cover trackerless hashes, DHT keeps peer counts fresh. After
Tier 3 lands the resolver actually resolves and trackers organically
converge across the swarm.

Anything not delivered by Tier 2 + Tier 3 (e.g. BEP 12 outer-dict tier
handling, scrape multitracker hint extension) is icing.
