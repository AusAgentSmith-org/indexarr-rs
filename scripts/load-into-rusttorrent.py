#!/usr/bin/env python3
"""
Load all torrents from Indexarr into rustTorrent.

Usage:
  python scripts/load-into-rusttorrent.py
  python scripts/load-into-rusttorrent.py --indexarr http://localhost:8080 --rusttorrent http://localhost:3030
  python scripts/load-into-rusttorrent.py --paused          # Add all torrents in paused state
  python scripts/load-into-rusttorrent.py --dry-run         # List torrents without adding
  python scripts/load-into-rusttorrent.py --min-seeders 1   # Only torrents with seeders
"""

import argparse
import json
import os
import sys
import time
import urllib.error
import urllib.request
from datetime import datetime

def log(msg, level="INFO"):
    ts = datetime.now().strftime("%H:%M:%S.%f")[:-3]
    print(f"[{ts}] [{level}] {msg}", flush=True)


def get_json(url, timeout=30):
    req = urllib.request.Request(url, headers={"Accept": "application/json"})
    with urllib.request.urlopen(req, timeout=timeout) as resp:
        return json.loads(resp.read())


def iter_torrents(indexarr_url, min_seeders=0, batch_size=100):
    """Yield torrents from Indexarr in batches."""
    offset = 0

    while True:
        url = f"{indexarr_url}/api/v1/search?limit={batch_size}&offset={offset}"
        if min_seeders > 0:
            url += f"&min_seeders={min_seeders}"

        data = get_json(url)
        results = data.get("results", [])
        total = data.get("total", 0)

        if not results:
            break

        yield results, offset, total

        if len(results) < batch_size:
            break
        offset += batch_size


def get_magnet(indexarr_url, info_hash):
    """Get magnet URI for a single torrent."""
    data = get_json(f"{indexarr_url}/api/v1/torrent/{info_hash}")
    return data.get("magnet_uri", "")


def add_to_rusttorrent(rusttorrent_url, magnet, paused=False, auth=None):
    """Add a magnet to rustTorrent."""
    url = f"{rusttorrent_url}/torrents"
    if paused:
        url += "?paused=true"

    headers = {"Content-Type": "text/plain"}
    if auth:
        import base64
        cred = base64.b64encode(f"{auth[0]}:{auth[1]}".encode()).decode()
        headers["Authorization"] = f"Basic {cred}"

    req = urllib.request.Request(
        url,
        data=magnet.encode("utf-8"),
        method="POST",
        headers=headers,
    )
    try:
        with urllib.request.urlopen(req, timeout=30) as resp:
            result = json.loads(resp.read())
            return result, resp.status
    except urllib.error.HTTPError as e:
        body = e.read().decode("utf-8", errors="replace")[:200]
        return body, e.code


def main():
    parser = argparse.ArgumentParser(description="Load Indexarr torrents into rustTorrent")
    parser.add_argument("--indexarr", default="http://localhost:8080", help="Indexarr URL")
    parser.add_argument("--rusttorrent", default="http://localhost:3030", help="rustTorrent URL")
    parser.add_argument("--paused", action="store_true", help="Add torrents in paused state")
    parser.add_argument("--dry-run", action="store_true", help="List torrents without adding")
    parser.add_argument("--min-seeders", type=int, default=0, help="Minimum seeders (default: 0)")
    parser.add_argument("--rt-user", type=str, default=os.environ.get("RT_USER", ""), help="rustTorrent username")
    parser.add_argument("--rt-pass", type=str, default=os.environ.get("RT_PASS", ""), help="rustTorrent password")
    parser.add_argument("--batch-size", type=int, default=100, help="Torrents per batch (default: 100)")
    parser.add_argument("--delay", type=float, default=0.1, help="Delay between adds in seconds (default: 0.1)")
    args = parser.parse_args()

    log(f"Indexarr:     {args.indexarr}")
    log(f"rustTorrent:  {args.rusttorrent}")
    log(f"Paused:       {args.paused}")
    log(f"Min seeders:  {args.min_seeders}")
    log(f"Batch size:   {args.batch_size}")

    auth = (args.rt_user, args.rt_pass) if args.rt_user else None
    if auth:
        log(f"Auth:         {auth[0]}")

    added = 0
    skipped = 0
    errors = 0
    processed = 0

    for batch, offset, total in iter_torrents(args.indexarr, args.min_seeders, args.batch_size):
        log(f"Batch {offset // args.batch_size + 1} — torrents {offset+1}-{offset+len(batch)} of {total}")

        for t in batch:
            processed += 1
            info_hash = t["info_hash"]
            name = t.get("name", "?")[:60]

            if args.dry_run:
                seeds = t.get("seed_count", 0)
                size_mb = (t.get("size", 0) or 0) / 1024 / 1024
                print(f"  {info_hash[:16]}  S:{seeds:>4}  {size_mb:>8.1f}MB  {name}")
                continue

            # Get magnet URI
            try:
                magnet = get_magnet(args.indexarr, info_hash)
            except Exception as e:
                log(f"[{processed}/{total}] Failed to get magnet for {info_hash[:16]}: {e}", "WARN")
                errors += 1
                continue

            if not magnet:
                log(f"[{processed}/{total}] No magnet for {info_hash[:16]} — skipping", "WARN")
                skipped += 1
                continue

            # Add to rustTorrent
            try:
                result, status = add_to_rusttorrent(args.rusttorrent, magnet, args.paused, auth)
                if isinstance(status, int) and 200 <= status < 300:
                    added += 1
                    if added % 25 == 0:
                        log(f"[{processed}/{total}] Added {added} so far — latest: {name}")
                else:
                    if isinstance(status, int) and status == 409:
                        skipped += 1
                    else:
                        log(f"[{processed}/{total}] HTTP {status} for {name}: {result}", "WARN")
                        errors += 1
            except Exception as e:
                log(f"[{processed}/{total}] Error adding {name}: {e}", "WARN")
                errors += 1

            if args.delay > 0:
                time.sleep(args.delay)

        if args.dry_run:
            log(f"Dry run — batch done ({processed}/{total})")
        else:
            log(f"Batch done — Added: {added}, Skipped: {skipped}, Errors: {errors}")

    log("=" * 60)
    if args.dry_run:
        log(f"Dry run — {processed} torrents would be added")
    else:
        log(f"Done — Added: {added}, Skipped: {skipped}, Errors: {errors}")


if __name__ == "__main__":
    main()
