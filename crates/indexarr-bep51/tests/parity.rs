//! BEP 51 parity tests, mirrored from
//! `pythonTorrentDHT/tests/test_bep51.py`.
//!
//! Each test below corresponds to a Python test of the same name; comments
//! cite the Python source for traceability.

use bencode::{ByteBufOwned, from_bytes};
use indexarr_bep51::{
    Bep51Error, COMPACT_NODE_V4_LEN, MAX_INTERVAL_SECS, MAX_SAMPLES, METHOD_NAME,
    SampleInfohashesArgs, SampleInfohashesResp, decode_query, decode_response, encode_query,
    encode_response, iter_samples, validate_response,
};
use librtbit_core::hash_id::Id20;
use serde::Deserialize;

fn id(byte: u8) -> Id20 {
    Id20::from_bytes(&[byte; 20]).unwrap()
}

// ─── TestBEP51SampleInfohashesQuery ─────────────────────────────────────

/// Mirrors `test_query_format`. Query must have `q=sample_infohashes`,
/// `a={id, target}`, and a transaction id `t`.
#[test]
fn test_query_format() {
    let args = SampleInfohashesArgs {
        id: id(0xaa),
        target: id(0xbb),
    };
    let buf = encode_query(b"\x01\x02", &args).unwrap();

    // Generic decode to inspect raw KRPC structure.
    #[derive(Deserialize)]
    struct Raw {
        #[serde(rename = "y")]
        y: ByteBufOwned,
        #[serde(rename = "t")]
        t: ByteBufOwned,
        #[serde(rename = "q")]
        q: ByteBufOwned,
        #[serde(rename = "a")]
        a: SampleInfohashesArgs,
    }
    let raw: Raw = from_bytes(&buf).unwrap();
    assert_eq!(raw.y.as_ref(), b"q");
    assert_eq!(raw.t.as_ref(), b"\x01\x02");
    assert_eq!(raw.q.as_ref(), METHOD_NAME);
    assert_eq!(raw.a.id, id(0xaa));
    assert_eq!(raw.a.target, id(0xbb));
}

/// Round-trip a query through encode → decode and confirm field preservation.
#[test]
fn test_query_round_trip() {
    let args = SampleInfohashesArgs {
        id: id(0x11),
        target: id(0x22),
    };
    let buf = encode_query(b"abcd", &args).unwrap();
    let (txid, decoded) = decode_query(&buf).unwrap().expect("is sample_infohashes");
    assert_eq!(txid.as_ref(), b"abcd");
    assert_eq!(decoded, args);
}

/// `decode_query` should return `None` for a non-`sample_infohashes` query.
#[test]
fn test_decode_query_ignores_other_methods() {
    use bencode::bencode_serialize_to_writer;
    use serde::Serialize;

    mod ser {
        use serde::Serializer;
        pub mod y_q {
            use super::Serializer;
            pub fn serialize<S: Serializer>(_: &(), s: S) -> Result<S::Ok, S::Error> {
                s.serialize_bytes(b"q")
            }
        }
        pub mod bytes {
            use super::Serializer;
            pub fn serialize<S: Serializer>(b: &[u8], s: S) -> Result<S::Ok, S::Error> {
                s.serialize_bytes(b)
            }
        }
    }

    #[derive(Serialize)]
    struct PingQuery<'a> {
        #[serde(rename = "y", with = "ser::y_q")]
        y: (),
        #[serde(rename = "t", with = "ser::bytes")]
        t: &'a [u8],
        #[serde(rename = "q", with = "ser::bytes")]
        q: &'a [u8],
        #[serde(rename = "a")]
        a: PingArgs,
    }
    #[derive(Serialize)]
    struct PingArgs {
        id: Id20,
    }
    let mut buf = Vec::new();
    let p = PingQuery {
        y: (),
        t: b"\x00\x01",
        q: b"ping",
        a: PingArgs { id: id(0x33) },
    };
    bencode_serialize_to_writer(p, &mut buf).unwrap();
    assert!(decode_query(&buf).unwrap().is_none());
}

// ─── TestBEP51SampleInfohashesResponse ──────────────────────────────────

/// Mirrors `test_response_has_required_fields`. All required keys present.
#[test]
fn test_response_has_required_fields() {
    let resp = SampleInfohashesResp::<ByteBufOwned> {
        id: id(0x01),
        interval: 60,
        nodes: ByteBufOwned::from(vec![0u8; COMPACT_NODE_V4_LEN]), // one compact node
        nodes6: None,
        num: 2,
        samples: ByteBufOwned::from(vec![0xdd; 20]),
    };
    let buf = encode_response(b"\x01\x02", &resp).unwrap();

    #[derive(Deserialize)]
    struct Raw {
        #[serde(rename = "r")]
        r: SampleInfohashesResp<ByteBufOwned>,
    }
    let raw: Raw = from_bytes(&buf).unwrap();
    assert_eq!(raw.r.id, id(0x01));
    assert_eq!(raw.r.interval, 60);
    assert!(!raw.r.nodes.as_ref().is_empty());
    assert_eq!(raw.r.num, 2);
    assert_eq!(raw.r.samples.as_ref(), &[0xdd; 20]);
}

/// Mirrors `test_response_nodes_are_compact`. nodes length is multiple of 26.
#[test]
fn test_response_nodes_are_compact() {
    let resp = SampleInfohashesResp::<ByteBufOwned> {
        id: id(0x01),
        interval: 60,
        nodes: ByteBufOwned::from(vec![0u8; COMPACT_NODE_V4_LEN * 3]),
        nodes6: None,
        num: 0,
        samples: ByteBufOwned::from(Vec::<u8>::new()),
    };
    validate_response(&resp).unwrap();
    assert_eq!(resp.nodes.as_ref().len() % COMPACT_NODE_V4_LEN, 0);
}

/// Validate path: nodes whose length isn't a multiple of 26 must reject.
#[test]
fn test_response_nodes_non_multiple_of_26_rejected() {
    let resp = SampleInfohashesResp::<ByteBufOwned> {
        id: id(0x01),
        interval: 60,
        nodes: ByteBufOwned::from(vec![0u8; 25]), // off by one
        nodes6: None,
        num: 0,
        samples: ByteBufOwned::from(Vec::<u8>::new()),
    };
    let err = validate_response(&resp).unwrap_err();
    assert!(matches!(err, Bep51Error::NodesNotMultipleOf26(25)));
}

/// Mirrors `test_response_interval_in_valid_range`.
#[test]
fn test_response_interval_in_valid_range() {
    let resp = SampleInfohashesResp::<ByteBufOwned> {
        id: id(0x01),
        interval: MAX_INTERVAL_SECS,
        nodes: ByteBufOwned::from(Vec::<u8>::new()),
        nodes6: None,
        num: 0,
        samples: ByteBufOwned::from(Vec::<u8>::new()),
    };
    validate_response(&resp).unwrap();

    // Just over → reject.
    let resp_bad = SampleInfohashesResp::<ByteBufOwned> {
        interval: MAX_INTERVAL_SECS + 1,
        ..resp
    };
    assert!(matches!(
        validate_response(&resp_bad),
        Err(Bep51Error::IntervalTooLarge(_))
    ));
}

/// Mirrors `test_response_samples_multiple_of_20`.
#[test]
fn test_response_samples_multiple_of_20() {
    let mut samples = Vec::new();
    samples.extend_from_slice(&[0xdd; 20]);
    samples.extend_from_slice(&[0xee; 20]);
    let resp = SampleInfohashesResp::<ByteBufOwned> {
        id: id(0x01),
        interval: 60,
        nodes: ByteBufOwned::from(Vec::<u8>::new()),
        nodes6: None,
        num: 2,
        samples: ByteBufOwned::from(samples.clone()),
    };
    validate_response(&resp).unwrap();
    assert_eq!(resp.samples.as_ref().len() % 20, 0);
    assert!(resp.samples.as_ref().len() >= 40);

    // Off-by-one rejected.
    let bad = SampleInfohashesResp::<ByteBufOwned> {
        samples: ByteBufOwned::from(vec![0xdd; 19]),
        ..resp
    };
    assert!(matches!(
        validate_response(&bad),
        Err(Bep51Error::SamplesNotMultipleOf20(19))
    ));
}

/// `samples` capped at MAX_SAMPLES (20 hashes = 400 bytes).
#[test]
fn test_response_samples_max_count_enforced() {
    let bad = SampleInfohashesResp::<ByteBufOwned> {
        id: id(0x01),
        interval: 60,
        nodes: ByteBufOwned::from(Vec::<u8>::new()),
        nodes6: None,
        num: 100,
        samples: ByteBufOwned::from(vec![0u8; 20 * (MAX_SAMPLES + 1)]),
    };
    assert!(matches!(
        validate_response(&bad),
        Err(Bep51Error::TooManySamples(_))
    ));
}

/// Mirrors `test_response_num_matches_stored_count`. We don't have a storage
/// layer here yet — just sanity-check that num is preserved through encode/decode.
#[test]
fn test_response_num_round_trip() {
    let resp = SampleInfohashesResp::<ByteBufOwned> {
        id: id(0x01),
        interval: 60,
        nodes: ByteBufOwned::from(Vec::<u8>::new()),
        nodes6: None,
        num: 12345,
        samples: ByteBufOwned::from(Vec::<u8>::new()),
    };
    let buf = encode_response(b"\x01\x02", &resp).unwrap();
    let (_, decoded) = decode_response(&buf).unwrap();
    assert_eq!(decoded.num, 12345);
}

/// Mirrors `test_response_empty_samples_when_no_peers`.
#[test]
fn test_response_empty_samples_when_no_peers() {
    let resp = SampleInfohashesResp::<ByteBufOwned> {
        id: id(0x01),
        interval: 60,
        nodes: ByteBufOwned::from(Vec::<u8>::new()),
        nodes6: None,
        num: 0,
        samples: ByteBufOwned::from(Vec::<u8>::new()),
    };
    let buf = encode_response(b"\x01\x02", &resp).unwrap();
    let (_, decoded) = decode_response(&buf).unwrap();
    assert!(decoded.samples.as_ref().is_empty());
    assert_eq!(decoded.num, 0);
}

/// Mirrors `test_transaction_id_echoed`. Caller is responsible for using the
/// query's `t` when building the response — verify our envelope preserves it.
#[test]
fn test_transaction_id_round_trip() {
    let resp = SampleInfohashesResp::<ByteBufOwned> {
        id: id(0x01),
        interval: 60,
        nodes: ByteBufOwned::from(Vec::<u8>::new()),
        nodes6: None,
        num: 0,
        samples: ByteBufOwned::from(Vec::<u8>::new()),
    };
    let buf = encode_response(b"\xaa\xbb", &resp).unwrap();
    let (txid, _) = decode_response(&buf).unwrap();
    assert_eq!(txid.as_ref(), b"\xaa\xbb");
}

// ─── TestBEP51MessageFields ─────────────────────────────────────────────

/// Mirrors `test_decode_response_with_bep51_fields`. Hand-craft a known-good
/// response and verify field-level decoding.
#[test]
fn test_decode_response_with_bep51_fields() {
    let resp = SampleInfohashesResp::<ByteBufOwned> {
        id: id(0x00),
        interval: 120,
        nodes: ByteBufOwned::from(vec![0u8; COMPACT_NODE_V4_LEN]),
        nodes6: None,
        num: 10,
        samples: ByteBufOwned::from(vec![0xbb; 60]), // 3 samples
    };
    let buf = encode_response(b"\x01\x02", &resp).unwrap();
    let (txid, decoded) = decode_response(&buf).unwrap();
    assert_eq!(txid.as_ref(), b"\x01\x02");
    assert_eq!(decoded.interval, 120);
    assert_eq!(decoded.num, 10);
    assert_eq!(decoded.samples.as_ref(), &[0xbb; 60][..]);
    assert_eq!(decoded.nodes.as_ref().len(), COMPACT_NODE_V4_LEN);
}

/// `iter_samples` yields each 20-byte hash from a concatenated samples blob.
#[test]
fn test_iter_samples() {
    let mut samples = Vec::new();
    samples.extend_from_slice(&[0x01; 20]);
    samples.extend_from_slice(&[0x02; 20]);
    samples.extend_from_slice(&[0x03; 20]);

    let collected: Vec<&[u8]> = iter_samples(&samples).collect();
    assert_eq!(collected.len(), 3);
    assert_eq!(collected[0], &[0x01; 20]);
    assert_eq!(collected[1], &[0x02; 20]);
    assert_eq!(collected[2], &[0x03; 20]);
}

/// `iter_samples` on a zero-length blob yields nothing.
#[test]
fn test_iter_samples_empty() {
    let collected: Vec<&[u8]> = iter_samples(&[]).collect();
    assert!(collected.is_empty());
}
