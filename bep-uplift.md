# BEP library uplift — `librtbit-*` to crates.io, parity with `btpydht`

Companion to `uplift.md`. That doc tracks the *application-level* roadmap
(get peer/tracker discovery working in indexarr-rs). This doc tracks the
*library* roadmap (turn rustTorrent's `librtbit-*` crate family into a
properly-published BEP toolkit on crates.io, with parity audited against
the existing Python `btpydht` library).

> Status: **draft, awaiting decisions** — see "Open decisions" at the end.
> Plan once decisions are locked.

---

## Background

Two BEP libraries already exist inside this org:

- **Python**: `pythonTorrentDHT` (Forgejo: `indexarr/pythonTorrentDHT`),
  importable as `btpydht`. ~5,300 LOC. Fork of nitmir/btdht extended for
  Python 3.10+. Used by Python `Indexarr` for DHT crawl + BEP 9 metadata
  fetch.
- **Rust**: 13-crate `librtbit-*` family (Forgejo org `indexarr`, repos
  `librtbit`, `librtbit-core`, `librtbit-bencode`, etc). Powers
  rustTorrent. Six members already on crates.io; the BEP-rich members
  (`peer-protocol`, `tracker-comms`) are not.

`indexarr-rs` only consumes `librtbit-dht` today. Tier 2 and Tier 3 of
`uplift.md` will pull in `librtbit-peer-protocol` and
`librtbit-tracker-comms` — which gives us a forcing function to publish
them properly.

---

## BEP coverage matrix

| BEP | Title | btpydht | librtbit | Notes |
|---|---|---|---|---|
| 5 | DHT Protocol | ✅ `dht.py` | ✅ `librtbit-dht` | parity |
| 9 | ut_metadata | ✅ `metadata.py:fetch_extended_from_peers` | ✅ `librtbit-peer-protocol::extended::ut_metadata` | parity |
| 10 | Extension Protocol | ✅ in `metadata.py` | ✅ `librtbit-peer-protocol::extended::handshake` | parity |
| 11 | PEX (`ut_pex`) | ❌ | ✅ `librtbit-peer-protocol::extended::ut_pex` | rust ahead |
| 12 | `announce-list` (multitracker) | partial (parsed in metadata) | likely ✅ in `librtbit-core::torrent_metainfo` | verify |
| 15 | UDP tracker scrape | ❌ (Python uses libtorrent for this) | ✅ `librtbit-tracker-comms::tracker_comms_udp` | rust ahead |
| 28 | Tracker Exchange (`lt_tex`) | ❌ | ❌ | **gap — both** |
| 51 | DHT infohash sampling (`sample_infohashes`) | ✅ `dht.py` + `tests/test_bep51.py` | ❌ | **gap — Rust missing** |

**Net new BEP work** for full parity-or-better:

- **BEP 51** in `librtbit-dht` — port from btpydht (DHT message + crawler hook).
- **BEP 28** in `librtbit-peer-protocol` — net-new in both stacks.

Everything else either already exists in librtbit or is Python-only (we don't
need to backport BEP 11/15 *to* btpydht).

---

## Crate inventory

| Crate | Forgejo version | crates.io | Hygiene flags |
|---|---|---|---|
| `librtbit` | 0.0.1 | ❌ | placeholder version; facade unclear |
| `librtbit-core` | 5.0.0 | ✅ | — |
| `librtbit-bencode` | 3.1.0 | ✅ | — |
| `librtbit-buffers` | 4.2.0 | ✅ | — |
| `librtbit-clone-to-owned` | 3.0.1 | ✅ | — |
| `librtbit-dht` | 5.3.0 | ✅ | needs BEP 51 |
| `librtbit-peer-protocol` | 4.3.0 | ❌ | needs BEP 28; needs publish |
| `librtbit-tracker-comms` | 3.0.0 | ❌ | **description is wrong** (says "sha1 implementations"); needs publish |
| `librtbit-sha1-wrapper` | 4.1.0 | ✅ | — |
| `librtbit-upnp` | 1.0.0 | ❌ | needs publish (or skip — not BEP-relevant) |
| `librtbit-upnp-serve` | 1.0.1 | ❌ | needs publish (or skip) |
| `librtbit-lsd` | 0.1.0 | ❌ | **description empty**; pre-1.0; defer publish |
| `rtbit` (binary) | 0.0.1 | ❌ | placeholder; CLI, defer |

---

## Phases

### Phase A — Audit & spec lock (½ day)

1. Clone all 13 `librtbit-*` Forgejo repos into `~/Working/Active/apps/libs/`
   (workspace convention — `~/Working/CLAUDE.md` lists `libs/` as the canonical
   shared-crates location). Already partially there — verify what's local.
2. Build the parity audit per BEP, line-by-line. Don't trust "exists in
   both" — compare:
   - Wire-format edge cases (truncated responses, malformed bencode, unknown
     extension IDs).
   - Concurrent-access semantics (Python is single-thread+asyncio; Rust is
     real parallelism).
   - Routing-table behaviour (bucket splits, K=8 vs K=20, ping cadence).
3. Lock in the **target API** for each crate as it'll appear on crates.io.
   Breaking changes happen now, not after publish.

### Phase B — Develop in indexarr-rs (1–2 days)

Recommended: **B2 (consume + new crates)** — depend on
`librtbit-peer-protocol = "4.3"` etc. directly from the Forgejo cargo
registry. Net new code lives as new crates in indexarr-rs:

- `crates/indexarr-bep51` — DHT `sample_infohashes` query/response + crawler
  hook. Wraps `librtbit-dht`. When stable, the *crate's content* moves into
  `librtbit-dht` itself; this crate gets retired.
- `crates/indexarr-bep28` — `lt_tex` extension message type. Mirrors the
  shape of `librtbit-peer-protocol::extended::ut_pex.rs`. When stable,
  moves into `librtbit-peer-protocol`.
- `crates/indexarr-resolver-v2` — the actual orchestrator that uses BEP 9
  from `librtbit-peer-protocol` to drive metadata fetch. Replaces the stub
  in `indexarr-dht::resolver`. Stays in indexarr-rs (it's app-layer).

Why B2:

- One canonical home per crate (rustTorrent's repo). No vendoring drift.
- New BEP work is each its own crate during development, so parity tests
  can be written before merging back.
- When backported, each becomes a discrete PR against rustTorrent's
  individual crate repos.

Alternative B1 (vendor) is viable if Forgejo cross-repo workflow is
painful, but creates two-master problem.

### Phase C — Crate hygiene & publish (½ day)

Per-crate checklist before `cargo publish` to crates.io:

- [ ] `description` is accurate (fix `librtbit-tracker-comms` and `librtbit-lsd` first)
- [ ] `keywords` (max 5) — picks: `bittorrent`, `dht`, `bep`, `p2p`, `protocol`
- [ ] `categories` — `network-programming`, `parsing`, `asynchronous`
- [ ] `repository = "https://github.com/Sprooty/rustTorrent"` (or equivalent)
- [ ] `homepage`, `documentation` URLs (docs.rs auto)
- [ ] `license` declared (MPL-2.0 to match upstream xmpp-rs convention? confirm)
- [ ] `readme = "README.md"` and per-crate README exists with:
  - One-paragraph "what this crate is"
  - BEP coverage table (from this doc)
  - Quick-start example matching the doctest
  - Stability + MSRV statement
- [ ] Public API doc-comments on every `pub fn`, `pub struct`, `pub enum`, `pub trait`
- [ ] At least one runnable example in `examples/` (e.g. `cargo run --example dht_get_peers`)
- [ ] Tests pass with `--no-default-features` and `--all-features`
- [ ] MSRV declared in Cargo.toml (`rust-version = "1.82"` or whatever the workspace uses)
- [ ] CHANGELOG.md entry with version bump rationale
- [ ] CI green on the source repo

Backport pipeline (each crate as discrete PR against its rustTorrent repo):

1. Open PR with new BEP modules + hygiene fixes.
2. Bump versions:
   - `librtbit-peer-protocol` → 4.4.0 (BEP 28 added; backwards-compatible)
   - `librtbit-dht` → 5.4.0 (BEP 51 added; backwards-compatible)
   - `librtbit-tracker-comms` → 3.1.0 (description fix; potentially API tidy)
3. `cargo publish` from each crate's `master` branch (after CI green) — first
   the deps, then dependents, in topological order.
4. Update `indexarr-rs/Cargo.toml` to depend on crates.io versions.
5. Retire `indexarr-bep51`, `indexarr-bep28` from indexarr-rs (or keep as
   thin re-exports for stability during transition).

### Phase D — indexarr-rs application work

Now `uplift.md`'s Tier 2/3 roadmap lands using polished public crates.
Order:

1. Tier 2.1 (BEP 15 UDP scrape via `librtbit-tracker-comms`).
2. Tier 2.2 (DHT peer-count refresher via `librtbit-dht`).
3. Tier 3.1 (BEP 9 metadata fetch via `librtbit-peer-protocol`).
4. Tier 3.2 (BEP 28 lt_tex — uses our new code).
5. Tier 3.3 (BEP 11 PEX — already in `librtbit-peer-protocol`, just enable).

---

## Topological dependency order (for publish)

Existing crates on crates.io publish first; downstream pulls fresh:

```
1. librtbit-clone-to-owned        (already published, 3.0.1)
2. librtbit-buffers                (already published, 4.2.0)
3. librtbit-bencode                (already published, 3.1.0)
4. librtbit-sha1-wrapper           (already published, 4.1.0)
5. librtbit-core                   (already published, 5.0.0)
6. librtbit-dht                    PUBLISH 5.4.0  (was 5.3.0; +BEP 51)
7. librtbit-peer-protocol          PUBLISH 4.4.0  (first crates.io publish; +BEP 28)
8. librtbit-tracker-comms          PUBLISH 3.1.0  (first crates.io publish; description fix)
9. librtbit-upnp                   PUBLISH 1.0.0  (first crates.io publish — optional, not BEP-related)
10. librtbit-upnp-serve            PUBLISH 1.0.1  (first crates.io publish — optional)
11. librtbit-lsd                   defer to 1.0.0; description + LSD spec writeup needed
12. librtbit                       defer; bump to real version once member crates are stable
13. rtbit                          defer; CLI, separate concern
```

---

## Open decisions

These need your call before the plan goes from "draft" to "execute":

1. **B1 (vendor) vs B2 (consume + new crates)** — recommend **B2**.
2. **License for new BEP work** — match the family. Need to confirm what
   the family currently uses (MPL-2.0? MIT/Apache?). Will check during
   Phase A audit. indexarr-rs itself stays AGPL-3.0; new generic library
   crates likely should be MPL-2.0 or MIT/Apache dual.
3. **crates.io namespace** — keep `librtbit-*`. Consistent with what's
   there, search-friendly.
4. **CHANGELOG location** — per-crate `CHANGELOG.md` in each rustTorrent
   crate repo (standard) — confirm.
5. **Backport target branch in rustTorrent** — `master` direct PR vs
   feature branch + cumulative release branch?
6. **Coordinate with rustTorrent's own roadmap** — is rustTorrent
   expecting these libraries to evolve in lock-step with its own client
   work? Need to talk to whoever owns rustTorrent (could be just you).

---

## Out of scope (for this uplift)

- Replacing `btpydht` for the Python Indexarr — Python Indexarr is now
  legacy (replaced by indexarr-rs on hertzde3). btpydht stays for any
  remaining consumers; we don't need to keep it in lockstep.
- Implementing BEP 12 (announce-list multitier) — already in
  `librtbit-core::torrent_metainfo` per audit, just not exercised. No
  publish blocker.
- `librtbit-lsd` (Local Service Discovery) — needs writeup before publish;
  not blocking BEP discovery work; defer to its own task.
- Rebranding away from `librtbit-*` namespace.
