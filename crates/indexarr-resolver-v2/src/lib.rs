//! BEP 9 metadata-fetch orchestrator.
//!
//! Drives the BitTorrent peer wire protocol end-to-end against a single
//! peer to retrieve the `info` dict for a known info_hash:
//!
//! 1. TCP connect.
//! 2. BitTorrent handshake (BEP 3).
//! 3. BEP 10 extended handshake — exchange `ut_metadata` IDs and learn
//!    the metadata size.
//! 4. BEP 9 — loop `ut_metadata` Request → Data per piece, accumulate.
//! 5. SHA1-verify the assembled metadata against the info_hash.
//!
//! Codec is provided by [`peer_protocol`](::peer_protocol)
//! (`librtbit-peer-protocol`); this crate is the orchestrator. Mirrors the
//! shape of `btpydht/metadata.py:217-340` from the Python audit reference.

use std::net::SocketAddr;
use std::time::Duration;

use bencode::bencode_serialize_to_writer;
use buffers::ByteBuf;
use librtbit_core::{constants::CHUNK_SIZE, hash_id::Id20};
use peer_protocol::{
    Handshake, Message, MessageDeserializeError, SerializeError,
    extended::{
        ExtendedMessage, PeerExtendedMessageIds, handshake::ExtendedHandshake,
        ut_metadata::UtMetadata,
    },
};
use sha1::{Digest, Sha1};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

/// Default per-peer timeout for the entire metadata fetch.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// Maximum metadata size we'll accept from a peer. 8 MiB ≈ ~250k file
/// torrents — well past any realistic value, guards against DoS.
pub const MAX_METADATA_SIZE: u32 = 8 * 1024 * 1024;

/// BEP 9 piece size (per spec, 16 KiB).
pub const METADATA_PIECE_SIZE: u32 = CHUNK_SIZE;

/// Errors raised by the metadata-fetch orchestrator.
#[derive(Debug, thiserror::Error)]
pub enum ResolverError {
    #[error("TCP connect failed: {0}")]
    Connect(#[source] std::io::Error),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("timeout after {0:?}")]
    Timeout(Duration),
    #[error("peer sent handshake with mismatched info_hash")]
    HandshakeMismatch,
    #[error("peer does not support BEP 10 extended messaging")]
    ExtendedNotSupported,
    #[error("peer's extended handshake omitted ut_metadata id")]
    NoUtMetadataId,
    #[error("peer's extended handshake omitted metadata_size")]
    NoMetadataSize,
    #[error("metadata_size {0} exceeds limit ({MAX_METADATA_SIZE})")]
    MetadataTooLarge(u32),
    #[error("metadata_size is zero")]
    EmptyMetadata,
    #[error("peer rejected piece {0}")]
    PieceRejected(u32),
    #[error("peer sent piece {received} but we requested {expected}")]
    PieceOutOfOrder { expected: u32, received: u32 },
    #[error("metadata SHA1 hash mismatch")]
    HashMismatch,
    #[error("malformed peer message: {0}")]
    PeerProtocol(#[from] MessageDeserializeError),
    #[error("serialize error: {0}")]
    Serialize(#[from] SerializeError),
    #[error("bencode serialize: {0}")]
    BencodeSerialize(#[from] bencode::SerializeError),
    #[error("connection closed by peer mid-fetch")]
    UnexpectedEof,
}

/// Configuration for a single fetch attempt.
#[derive(Debug, Clone, Copy)]
pub struct FetchConfig {
    pub timeout: Duration,
    pub max_metadata_size: u32,
}

impl Default for FetchConfig {
    fn default() -> Self {
        Self {
            timeout: DEFAULT_TIMEOUT,
            max_metadata_size: MAX_METADATA_SIZE,
        }
    }
}

/// Result of a successful fetch.
#[derive(Debug)]
pub struct FetchedMetadata {
    /// Raw bencoded `info` dict bytes. SHA1 of this == info_hash.
    pub bytes: Vec<u8>,
}

/// Generate a fresh peer_id with the standard rtbit-style prefix.
pub fn random_peer_id() -> Id20 {
    use rand::RngCore;
    let mut bytes = [0u8; 20];
    bytes[..8].copy_from_slice(b"-IDXR01-");
    rand::thread_rng().fill_bytes(&mut bytes[8..]);
    Id20::new(bytes)
}

/// Fetch BEP 9 metadata for `info_hash` from a single peer at `peer_addr`.
///
/// On success returns the raw bencoded `info` dict bytes. The bytes are
/// SHA1-verified to match `info_hash` before returning — caller can trust
/// the payload is authentic.
pub async fn fetch_from_peer(
    info_hash: Id20,
    peer_addr: SocketAddr,
    peer_id: Id20,
    config: FetchConfig,
) -> Result<FetchedMetadata, ResolverError> {
    let task = fetch_inner(info_hash, peer_addr, peer_id);
    match tokio::time::timeout(config.timeout, task).await {
        Ok(result) => {
            let mut metadata = result?;
            verify_size(metadata.bytes.len(), config.max_metadata_size)?;
            verify_hash(&metadata.bytes, info_hash)?;
            // Defensive: shrink to capacity = len so the caller doesn't carry
            // a giant pre-allocated buffer around.
            metadata.bytes.shrink_to_fit();
            Ok(metadata)
        }
        Err(_) => Err(ResolverError::Timeout(config.timeout)),
    }
}

fn verify_size(actual: usize, max: u32) -> Result<(), ResolverError> {
    if actual as u64 > max as u64 {
        return Err(ResolverError::MetadataTooLarge(actual as u32));
    }
    Ok(())
}

fn verify_hash(metadata: &[u8], info_hash: Id20) -> Result<(), ResolverError> {
    let mut h = Sha1::new();
    h.update(metadata);
    let digest: [u8; 20] = h.finalize().into();
    if digest == info_hash.0 {
        Ok(())
    } else {
        Err(ResolverError::HashMismatch)
    }
}

async fn fetch_inner(
    info_hash: Id20,
    peer_addr: SocketAddr,
    peer_id: Id20,
) -> Result<FetchedMetadata, ResolverError> {
    let mut stream = TcpStream::connect(peer_addr)
        .await
        .map_err(ResolverError::Connect)?;

    // ─── BitTorrent handshake (BEP 3) ────────────────────────────────────
    let our_handshake = Handshake::new(info_hash, peer_id);
    let mut hs_buf = [0u8; 68];
    let n = our_handshake.serialize_unchecked_len(&mut hs_buf);
    debug_assert_eq!(n, 68);
    stream.write_all(&hs_buf).await?;

    let mut their_hs_buf = [0u8; 68];
    stream.read_exact(&mut their_hs_buf).await?;
    let (their_hs, _) = Handshake::deserialize(&their_hs_buf)?;
    if their_hs.info_hash != info_hash {
        return Err(ResolverError::HandshakeMismatch);
    }
    if !their_hs.supports_extended() {
        return Err(ResolverError::ExtendedNotSupported);
    }
    tracing::trace!(?peer_addr, ?info_hash, "bt handshake ok");

    // ─── BEP 10 extended handshake ──────────────────────────────────────
    send_extended_handshake(&mut stream).await?;
    let (ut_metadata_id, total_size) = recv_extended_handshake(&mut stream).await?;
    if total_size == 0 {
        return Err(ResolverError::EmptyMetadata);
    }
    if total_size > MAX_METADATA_SIZE {
        return Err(ResolverError::MetadataTooLarge(total_size));
    }
    tracing::trace!(?peer_addr, ut_metadata_id, total_size, "ext handshake ok");

    // ─── BEP 9 piece loop ──────────────────────────────────────────────
    let total_pieces = total_size.div_ceil(METADATA_PIECE_SIZE);
    let mut metadata = vec![0u8; total_size as usize];
    let peer_ids = PeerExtendedMessageIds {
        ut_metadata: Some(ut_metadata_id),
        ..Default::default()
    };

    for piece in 0..total_pieces {
        send_metadata_request(&mut stream, piece, peer_ids).await?;
        let chunk = recv_metadata_data(&mut stream, piece).await?;
        let offset = (piece as usize) * METADATA_PIECE_SIZE as usize;
        let end = offset + chunk.len();
        if end > metadata.len() {
            return Err(ResolverError::MetadataTooLarge(end as u32));
        }
        metadata[offset..end].copy_from_slice(&chunk);
        tracing::trace!(piece, len = chunk.len(), "ut_metadata piece ok");
    }

    Ok(FetchedMetadata { bytes: metadata })
}

async fn send_extended_handshake(stream: &mut TcpStream) -> Result<(), ResolverError> {
    // Build the bencoded extended-handshake payload directly (avoids needing
    // the peer's ut_metadata id, which we don't know yet).
    let mut payload = Vec::with_capacity(64);
    let h = ExtendedHandshake::<ByteBuf<'_>> {
        m: PeerExtendedMessageIds::my(),
        ..Default::default()
    };
    bencode_serialize_to_writer(&h, &mut payload)?;

    // Wire layout: 4-byte length prefix | msg_id (20 = MSGID_EXTENDED) | emsg_id (0 = handshake) | payload
    let body_len = 1 + 1 + payload.len();
    let total_len = 4 + body_len;
    let mut frame = vec![0u8; total_len];
    frame[0..4].copy_from_slice(&(body_len as u32).to_be_bytes());
    frame[4] = MSGID_EXTENDED;
    frame[5] = 0; // ext-handshake message-type id
    frame[6..].copy_from_slice(&payload);
    stream.write_all(&frame).await?;
    Ok(())
}

async fn recv_extended_handshake(stream: &mut TcpStream) -> Result<(u8, u32), ResolverError> {
    loop {
        let frame = read_frame(stream).await?;
        let (msg, _) = Message::deserialize(&frame, &[])?;
        match msg {
            Message::Extended(ExtendedMessage::Handshake(eh)) => {
                let m = eh.peer_extended_messages();
                let id = m.ut_metadata.ok_or(ResolverError::NoUtMetadataId)?;
                let size = eh.metadata_size.ok_or(ResolverError::NoMetadataSize)?;
                return Ok((id, size));
            }
            // Tolerate any non-extended-handshake message that arrives first
            // (Bitfield, Have, KeepAlive, choke/unchoke from over-eager peers,
            // etc.). The Extended-but-not-Handshake case is also fine here —
            // peers shouldn't send other extended messages before they've
            // received our handshake, but if they do we just skip them.
            other => {
                tracing::trace!(?other, "ignoring pre-extended-handshake message");
                continue;
            }
        }
    }
}

async fn send_metadata_request(
    stream: &mut TcpStream,
    piece: u32,
    peer_ids: PeerExtendedMessageIds,
) -> Result<(), ResolverError> {
    // Wire frame: 4-byte len prefix + 1 byte msg_id + 1 byte emsg_id + bencoded Request dict.
    // peer_protocol's `Message::serialize` reads the peer's ut_metadata id from
    // the `peer_extended_messages` callback so we don't need to pass it explicitly.
    let msg = Message::Extended(ExtendedMessage::UtMetadata(UtMetadata::Request(piece)));
    // 128 bytes is plenty for the worst-case Request frame (~30 bytes actual).
    let mut buf = vec![0u8; 128];
    let n = msg.serialize(&mut buf, &|| peer_ids)?;
    stream.write_all(&buf[..n]).await?;
    Ok(())
}

async fn recv_metadata_data(
    stream: &mut TcpStream,
    expected_piece: u32,
) -> Result<Vec<u8>, ResolverError> {
    loop {
        let frame = read_frame(stream).await?;
        let (msg, _) = Message::deserialize(&frame, &[])?;
        match msg {
            Message::Extended(ExtendedMessage::UtMetadata(UtMetadata::Data(d))) => {
                if d.piece() != expected_piece {
                    return Err(ResolverError::PieceOutOfOrder {
                        expected: expected_piece,
                        received: d.piece(),
                    });
                }
                let mut out = vec![0u8; d.len()];
                d.copy_to_slice(&mut out);
                return Ok(out);
            }
            Message::Extended(ExtendedMessage::UtMetadata(UtMetadata::Reject(p))) => {
                return Err(ResolverError::PieceRejected(p));
            }
            // Tolerate non-metadata traffic interleaved with our fetch.
            _ => continue,
        }
    }
}

/// Read a single length-prefixed peer-protocol frame: `[4-byte BE len][body]`.
/// Returns the full frame including the prefix, ready to feed into
/// `Message::deserialize`.
async fn read_frame(stream: &mut TcpStream) -> Result<Vec<u8>, ResolverError> {
    let mut len_buf = [0u8; 4];
    stream
        .read_exact(&mut len_buf)
        .await
        .map_err(eof_to_resolver)?;
    let body_len = u32::from_be_bytes(len_buf) as usize;
    let mut frame = vec![0u8; 4 + body_len];
    frame[0..4].copy_from_slice(&len_buf);
    if body_len > 0 {
        stream
            .read_exact(&mut frame[4..])
            .await
            .map_err(eof_to_resolver)?;
    }
    Ok(frame)
}

fn eof_to_resolver(e: std::io::Error) -> ResolverError {
    if e.kind() == std::io::ErrorKind::UnexpectedEof {
        ResolverError::UnexpectedEof
    } else {
        ResolverError::Io(e)
    }
}

const MSGID_EXTENDED: u8 = 20;
