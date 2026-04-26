use std::collections::VecDeque;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use dht::Id20;
use indexarr_bep51::{
    COMPACT_NODE_V4_LEN, SampleInfohashesArgs, decode_response, encode_query, iter_samples,
};
use tokio::net::UdpSocket;
use tokio_util::sync::CancellationToken;

use crate::{DhtSharedState, DiscoveredHash};

/// Well-known DHT bootstrap nodes — used to seed the BEP 51 sampler's node
/// queue on startup and after queue exhaustion.
const BOOTSTRAP: &[&str] = &[
    "dht.transmissionbt.com:6881",
    "dht.libtorrent.org:25401",
    "router.utorrent.com:6881",
    "router.bittorrent.com:6881",
];

/// Maximum number of nodes held in the sampler's query queue.
const MAX_QUEUE: usize = 5_000;

/// Delay between individual sample_infohashes queries to avoid flooding the
/// DHT network.
const INTER_QUERY_MS: u64 = 100;

/// Per-query response timeout.
const QUERY_TIMEOUT: Duration = Duration::from_secs(5);

/// Run the BEP 51 DHT infohash sampler.
///
/// Opens its own UDP socket and periodically issues `sample_infohashes` KRPC
/// queries (BEP 51) to known DHT nodes, feeding discovered info_hashes into
/// `shared` and expanding the node queue from each response's `nodes` field.
pub async fn run_bep51_sampler(shared: Arc<DhtSharedState>, cancel: CancellationToken) {
    // Wait for the DHT to warm up before we start hammering nodes.
    tokio::time::sleep(Duration::from_secs(45)).await;

    let socket = match UdpSocket::bind("0.0.0.0:0").await {
        Ok(s) => s,
        Err(e) => {
            tracing::error!(error = %e, "BEP 51 sampler: failed to bind UDP socket");
            return;
        }
    };
    tracing::info!(
        addr = %socket.local_addr().unwrap_or_else(|_| "0.0.0.0:0".parse().unwrap()),
        "BEP 51 sampler started"
    );

    let my_id = Id20::new(rand::random());
    let mut node_queue: VecDeque<SocketAddr> = VecDeque::new();
    let mut txn_counter: u16 = 0;
    let mut recv_buf = vec![0u8; 4096];

    seed_queue(&mut node_queue).await;

    let mut total_samples: u64 = 0;

    loop {
        if cancel.is_cancelled() {
            break;
        }

        // Refill queue from bootstrap if exhausted.
        if node_queue.is_empty() {
            tracing::debug!("BEP 51 sampler: node queue empty, reseeding");
            seed_queue(&mut node_queue).await;
            if node_queue.is_empty() {
                tokio::time::sleep(Duration::from_secs(10)).await;
                continue;
            }
        }

        let node = match node_queue.pop_front() {
            Some(n) => n,
            None => continue,
        };

        txn_counter = txn_counter.wrapping_add(1);
        let txn_id = txn_counter.to_be_bytes();
        let target = Id20::new(rand::random());
        let args = SampleInfohashesArgs { id: my_id, target };

        let query = match encode_query(&txn_id, &args) {
            Ok(q) => q,
            Err(e) => {
                tracing::trace!(error = %e, "BEP 51: encode_query failed");
                continue;
            }
        };

        if socket.send_to(&query, node).await.is_err() {
            continue;
        }

        let recv_result =
            tokio::time::timeout(QUERY_TIMEOUT, socket.recv_from(&mut recv_buf)).await;

        if let Ok(Ok((len, _src))) = recv_result {
            match decode_response(&recv_buf[..len]) {
                Ok((_t, resp)) => {
                    // Push discovered info_hashes into the shared ingest queue.
                    let mut count = 0usize;
                    for hash_bytes in iter_samples(resp.samples.as_ref()) {
                        let hex = hex::encode(hash_bytes);
                        shared.push_hash(DiscoveredHash {
                            info_hash: hex,
                            peer_ip: None,
                            peer_port: None,
                            source: "bep51".to_string(),
                        });
                        count += 1;
                    }
                    total_samples += count as u64;

                    // Expand the node queue from the response's compact node list.
                    let nodes = resp.nodes.as_ref();
                    let node_count = nodes.len() / COMPACT_NODE_V4_LEN;
                    for i in 0..node_count {
                        let base = i * COMPACT_NODE_V4_LEN + 20; // skip 20-byte node id
                        if base + 6 > nodes.len() {
                            break;
                        }
                        let ip = Ipv4Addr::new(
                            nodes[base],
                            nodes[base + 1],
                            nodes[base + 2],
                            nodes[base + 3],
                        );
                        let port = u16::from_be_bytes([nodes[base + 4], nodes[base + 5]]);
                        if port > 0 && node_queue.len() < MAX_QUEUE {
                            node_queue.push_back(SocketAddr::from((ip, port)));
                        }
                    }

                    if count > 0 {
                        tracing::debug!(
                            samples = count,
                            nodes = node_count,
                            total = total_samples,
                            "BEP 51: response"
                        );
                    }
                }
                // Most failures here are error responses from nodes that
                // don't implement BEP 51 — safe to ignore.
                Err(e) => tracing::trace!(error = %e, "BEP 51: decode error"),
            }
        }

        tokio::time::sleep(Duration::from_millis(INTER_QUERY_MS)).await;
    }

    tracing::info!(total_samples, "BEP 51 sampler stopped");
}

async fn seed_queue(queue: &mut VecDeque<SocketAddr>) {
    for host in BOOTSTRAP {
        match tokio::net::lookup_host(host).await {
            Ok(addrs) => {
                for addr in addrs {
                    if queue.len() < MAX_QUEUE {
                        queue.push_back(addr);
                    }
                }
            }
            Err(e) => tracing::trace!(host, error = %e, "BEP 51: bootstrap lookup failed"),
        }
    }
}
