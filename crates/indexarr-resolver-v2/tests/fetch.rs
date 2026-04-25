//! End-to-end tests against a mock BitTorrent peer running on localhost.
//! Each test spawns a tokio task that speaks the peer wire protocol and
//! serves a known metadata blob, then asserts the resolver retrieves it.

use std::time::Duration;

use bencode::bencode_serialize_to_writer;
use buffers::ByteBuf;
use indexarr_resolver_v2::{
    FetchConfig, MAX_METADATA_SIZE, METADATA_PIECE_SIZE, ResolverError, fetch_from_peer,
    random_peer_id,
};
use librtbit_core::hash_id::Id20;
use peer_protocol::{
    Handshake, Message,
    extended::{
        ExtendedMessage, PeerExtendedMessageIds,
        handshake::ExtendedHandshake,
        ut_metadata::{UtMetadata, UtMetadataData},
    },
};
use sha1::{Digest, Sha1};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    task::JoinHandle,
};

// Must match `librtbit_peer_protocol::MY_EXTENDED_UT_METADATA` — the library
// hardcodes this in both `PeerExtendedMessageIds::my()` *and* the deserializer's
// emsg_id match arm, so any peer built on this library MUST advertise this
// value for its own ut_metadata extension.
const PEER_UT_METADATA_ID: u8 = 3;

fn sha1(data: &[u8]) -> Id20 {
    let mut h = Sha1::new();
    h.update(data);
    Id20::new(h.finalize().into())
}

/// Build a fake metadata blob of `len` bytes and return (info_hash, blob).
fn make_metadata(len: usize) -> (Id20, Vec<u8>) {
    let blob: Vec<u8> = (0..len).map(|i| (i & 0xff) as u8).collect();
    (sha1(&blob), blob)
}

/// Run a bare-bones BitTorrent peer that serves `metadata` for `info_hash`.
/// Optional knobs let individual tests inject failure modes.
#[derive(Default)]
struct MockPeerOpts {
    /// If true, send back a different info_hash in our handshake.
    handshake_mismatch: bool,
    /// If true, omit `metadata_size` from the extended handshake.
    omit_metadata_size: bool,
    /// If true, reject every ut_metadata Request with a Reject message.
    always_reject: bool,
    /// If true, advertise a metadata_size larger than MAX_METADATA_SIZE.
    advertise_oversized: bool,
}

async fn run_mock_peer(
    listener: TcpListener,
    info_hash: Id20,
    metadata: Vec<u8>,
    opts: MockPeerOpts,
) {
    let (mut sock, _) = listener.accept().await.unwrap();
    if let Err(e) = serve_one_peer(&mut sock, info_hash, metadata, opts).await {
        eprintln!("MOCK PEER ERROR: {e:?}");
    }
}

async fn serve_one_peer(
    sock: &mut TcpStream,
    info_hash: Id20,
    metadata: Vec<u8>,
    opts: MockPeerOpts,
) -> Result<(), Box<dyn std::error::Error>> {
    // ── BT handshake ──
    let mut buf = [0u8; 68];
    sock.read_exact(&mut buf).await?;
    let (their_hs, _) = Handshake::deserialize(&buf)?;

    let our_hs = if opts.handshake_mismatch {
        Handshake::new(sha1(b"different-info-hash"), random_peer_id())
    } else {
        Handshake::new(their_hs.info_hash, random_peer_id())
    };
    let mut hs_out = [0u8; 68];
    let _ = our_hs.serialize_unchecked_len(&mut hs_out);
    sock.write_all(&hs_out).await?;

    if opts.handshake_mismatch {
        return Ok(());
    }
    let _ = info_hash;

    // ── Send extended handshake ──
    let advertised_size = if opts.advertise_oversized {
        MAX_METADATA_SIZE + 1
    } else {
        metadata.len() as u32
    };
    let mut payload = Vec::new();
    let mut h = ExtendedHandshake::<ByteBuf<'_>> {
        m: PeerExtendedMessageIds {
            ut_metadata: Some(PEER_UT_METADATA_ID),
            ..Default::default()
        },
        ..Default::default()
    };
    if !opts.omit_metadata_size {
        h.metadata_size = Some(advertised_size);
    }
    bencode_serialize_to_writer(&h, &mut payload)?;
    write_extended_handshake(sock, &payload).await?;

    if opts.omit_metadata_size {
        return Ok(());
    }

    // ── Serve ut_metadata pieces in response to Requests ──
    loop {
        let frame = match read_frame(sock).await {
            Ok(f) => f,
            Err(_) => return Ok(()),
        };
        let (msg, _) = Message::deserialize(&frame, &[])?;
        if let Message::Extended(ExtendedMessage::UtMetadata(UtMetadata::Request(piece))) = msg {
            if opts.always_reject {
                write_ut_metadata_reject(sock, piece).await?;
                continue;
            }
            let offset = (piece as usize) * METADATA_PIECE_SIZE as usize;
            let end = ((offset + METADATA_PIECE_SIZE as usize).min(metadata.len())).max(offset);
            let chunk = &metadata[offset..end];
            write_ut_metadata_data(sock, piece, metadata.len() as u32, chunk).await?;
        }
    }
}

async fn write_extended_handshake(sock: &mut TcpStream, payload: &[u8]) -> std::io::Result<()> {
    let body_len = 1 + 1 + payload.len();
    let total_len = 4 + body_len;
    let mut frame = vec![0u8; total_len];
    frame[0..4].copy_from_slice(&(body_len as u32).to_be_bytes());
    frame[4] = 20; // MSGID_EXTENDED
    frame[5] = 0; // ext-handshake
    frame[6..].copy_from_slice(payload);
    sock.write_all(&frame).await
}

async fn write_ut_metadata_reject(sock: &mut TcpStream, piece: u32) -> std::io::Result<()> {
    let msg = Message::Extended(ExtendedMessage::UtMetadata(UtMetadata::Reject(piece)));
    let mut buf = vec![0u8; 128];
    let n = msg
        .serialize(&mut buf, &|| PeerExtendedMessageIds {
            ut_metadata: Some(PEER_UT_METADATA_ID),
            ..Default::default()
        })
        .unwrap();
    sock.write_all(&buf[..n]).await
}

async fn write_ut_metadata_data(
    sock: &mut TcpStream,
    piece: u32,
    total_size: u32,
    chunk: &[u8],
) -> std::io::Result<()> {
    let msg = Message::Extended(ExtendedMessage::UtMetadata(UtMetadata::Data(
        UtMetadataData::from_bytes(piece, total_size, ByteBuf::from(chunk)),
    )));
    let mut buf = vec![0u8; chunk.len() + 256];
    let n = msg
        .serialize(&mut buf, &|| PeerExtendedMessageIds {
            ut_metadata: Some(PEER_UT_METADATA_ID),
            ..Default::default()
        })
        .unwrap();
    sock.write_all(&buf[..n]).await
}

async fn read_frame(sock: &mut TcpStream) -> std::io::Result<Vec<u8>> {
    let mut len_buf = [0u8; 4];
    sock.read_exact(&mut len_buf).await?;
    let body_len = u32::from_be_bytes(len_buf) as usize;
    let mut frame = vec![0u8; 4 + body_len];
    frame[0..4].copy_from_slice(&len_buf);
    if body_len > 0 {
        sock.read_exact(&mut frame[4..]).await?;
    }
    Ok(frame)
}

async fn spawn_mock(
    info_hash: Id20,
    metadata: Vec<u8>,
    opts: MockPeerOpts,
) -> (JoinHandle<()>, std::net::SocketAddr) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let h = tokio::spawn(run_mock_peer(listener, info_hash, metadata, opts));
    (h, addr)
}

// ─── Tests ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn fetch_single_piece_metadata() {
    // 4 KiB → fits in a single 16 KiB BEP 9 piece.
    let (info_hash, metadata) = make_metadata(4096);
    let (handle, addr) = spawn_mock(info_hash, metadata.clone(), MockPeerOpts::default()).await;

    let result = fetch_from_peer(info_hash, addr, random_peer_id(), FetchConfig::default()).await;
    let fetched = result.expect("fetch ok");
    assert_eq!(fetched.bytes, metadata);

    handle.abort();
}

#[tokio::test]
async fn fetch_multi_piece_metadata() {
    // ~40 KiB → 3 pieces (16 KiB + 16 KiB + ~8 KiB).
    let (info_hash, metadata) = make_metadata(40_000);
    let (handle, addr) = spawn_mock(info_hash, metadata.clone(), MockPeerOpts::default()).await;

    let result = fetch_from_peer(info_hash, addr, random_peer_id(), FetchConfig::default()).await;
    let fetched = result.expect("fetch ok");
    assert_eq!(fetched.bytes.len(), 40_000);
    assert_eq!(fetched.bytes, metadata);

    handle.abort();
}

#[tokio::test]
async fn fetch_handshake_mismatch_rejected() {
    let (info_hash, metadata) = make_metadata(4096);
    let (handle, addr) = spawn_mock(
        info_hash,
        metadata,
        MockPeerOpts {
            handshake_mismatch: true,
            ..Default::default()
        },
    )
    .await;

    let err = fetch_from_peer(info_hash, addr, random_peer_id(), FetchConfig::default())
        .await
        .unwrap_err();
    assert!(matches!(err, ResolverError::HandshakeMismatch));

    handle.abort();
}

#[tokio::test]
async fn fetch_missing_metadata_size_rejected() {
    let (info_hash, metadata) = make_metadata(4096);
    let (handle, addr) = spawn_mock(
        info_hash,
        metadata,
        MockPeerOpts {
            omit_metadata_size: true,
            ..Default::default()
        },
    )
    .await;

    let err = fetch_from_peer(info_hash, addr, random_peer_id(), FetchConfig::default())
        .await
        .unwrap_err();
    assert!(matches!(err, ResolverError::NoMetadataSize));

    handle.abort();
}

#[tokio::test]
async fn fetch_oversized_metadata_rejected_at_handshake() {
    let (info_hash, metadata) = make_metadata(4096);
    let (handle, addr) = spawn_mock(
        info_hash,
        metadata,
        MockPeerOpts {
            advertise_oversized: true,
            ..Default::default()
        },
    )
    .await;

    let err = fetch_from_peer(info_hash, addr, random_peer_id(), FetchConfig::default())
        .await
        .unwrap_err();
    assert!(matches!(err, ResolverError::MetadataTooLarge(_)));

    handle.abort();
}

#[tokio::test]
async fn fetch_reject_returns_piece_rejected_error() {
    let (info_hash, metadata) = make_metadata(4096);
    let (handle, addr) = spawn_mock(
        info_hash,
        metadata,
        MockPeerOpts {
            always_reject: true,
            ..Default::default()
        },
    )
    .await;

    let err = fetch_from_peer(info_hash, addr, random_peer_id(), FetchConfig::default())
        .await
        .unwrap_err();
    assert!(matches!(err, ResolverError::PieceRejected(0)));

    handle.abort();
}

#[tokio::test]
async fn fetch_hash_mismatch_rejected() {
    // Mock peer serves `metadata` whose SHA1 does NOT match the info_hash we ask for.
    let (real_info_hash, metadata) = make_metadata(4096);
    let fake_info_hash = sha1(b"this-is-not-the-real-info-hash");

    let (handle, addr) = spawn_mock(real_info_hash, metadata, MockPeerOpts::default()).await;

    // The mock peer accepts ANY info_hash in its handshake (echoes ours back),
    // so handshake/metadata transfer succeeds — but the SHA1 verification at
    // the end catches the mismatch.
    let err = fetch_from_peer(
        fake_info_hash,
        addr,
        random_peer_id(),
        FetchConfig::default(),
    )
    .await
    .unwrap_err();
    assert!(matches!(err, ResolverError::HashMismatch));

    handle.abort();
}

#[tokio::test]
async fn fetch_timeout_when_peer_silent() {
    // Peer accepts the connection but never speaks.
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let _silent = tokio::spawn(async move {
        let (_sock, _) = listener.accept().await.unwrap();
        tokio::time::sleep(Duration::from_secs(5)).await;
    });

    let (info_hash, _) = make_metadata(4096);
    let cfg = FetchConfig {
        timeout: Duration::from_millis(200),
        ..Default::default()
    };
    let err = fetch_from_peer(info_hash, addr, random_peer_id(), cfg)
        .await
        .unwrap_err();
    assert!(matches!(err, ResolverError::Timeout(_)));
}

#[tokio::test]
async fn fetch_connect_refused() {
    // 127.0.0.1:1 is virtually guaranteed to refuse connections (no listener).
    let (info_hash, _) = make_metadata(4096);
    let addr: std::net::SocketAddr = "127.0.0.1:1".parse().unwrap();
    let cfg = FetchConfig {
        timeout: Duration::from_secs(1),
        ..Default::default()
    };
    let err = fetch_from_peer(info_hash, addr, random_peer_id(), cfg)
        .await
        .unwrap_err();
    assert!(matches!(err, ResolverError::Connect(_)));
}
