//! BEP 28 (lt_tex) round-trip and edge-case tests. No reference test suite
//! exists upstream — these tests exercise each documented invariant of the
//! BEP and a few practical edge cases.

use indexarr_bep28::{Bep28Error, FLAG_VERIFIED, LtTex, METHOD_NAME, decode, encode};

#[test]
fn method_name_is_lt_tex() {
    assert_eq!(METHOD_NAME, b"lt_tex");
}

#[test]
fn empty_message_round_trips() {
    let msg = LtTex::default();
    assert!(msg.is_empty());
    assert_eq!(msg.len(), 0);
    let buf = encode(&msg).unwrap();
    let decoded = decode(&buf).unwrap();
    assert!(decoded.is_empty());
}

#[test]
fn single_tracker_no_flags_round_trips() {
    let msg = LtTex::from_trackers(["http://tracker1.example/announce"], None);
    let buf = encode(&msg).unwrap();
    let decoded = decode(&buf).unwrap();
    let trackers: Vec<_> = decoded.iter().collect();
    assert_eq!(trackers.len(), 1);
    assert_eq!(trackers[0].0, b"http://tracker1.example/announce");
    assert_eq!(trackers[0].1, 0); // no flags → 0
}

#[test]
fn multiple_trackers_with_flags_round_trip() {
    let msg = LtTex::from_trackers(
        [
            "http://tracker1.example/announce",
            "udp://tracker2.example:1337/announce",
            "http://tracker3.example/announce",
        ],
        Some(vec![FLAG_VERIFIED, 0, FLAG_VERIFIED]),
    );
    let buf = encode(&msg).unwrap();
    let decoded = decode(&buf).unwrap();
    assert_eq!(decoded.len(), 3);
    let trackers: Vec<_> = decoded.iter().collect();
    assert_eq!(trackers[0].1, FLAG_VERIFIED);
    assert_eq!(trackers[1].1, 0);
    assert_eq!(trackers[2].1, FLAG_VERIFIED);
    assert_eq!(trackers[0].0, b"http://tracker1.example/announce");
    assert_eq!(trackers[1].0, b"udp://tracker2.example:1337/announce");
    assert_eq!(trackers[2].0, b"http://tracker3.example/announce");
}

#[test]
fn flags_length_mismatch_rejected_on_encode() {
    let msg = LtTex::from_trackers(["http://a/announce", "http://b/announce"], Some(vec![1])); // 2 vs 1
    let err = encode(&msg).unwrap_err();
    assert!(matches!(
        err,
        Bep28Error::FlagsLengthMismatch {
            added_len: 2,
            flags_len: 1
        }
    ));
}

#[test]
fn flags_present_without_added_rejected() {
    let msg = LtTex {
        added: None,
        added_f: Some(bencode::ByteBufOwned::from(vec![1, 0])),
    };
    let err = encode(&msg).unwrap_err();
    assert!(matches!(
        err,
        Bep28Error::FlagsLengthMismatch {
            added_len: 0,
            flags_len: 2
        }
    ));
}

#[test]
fn unknown_extra_keys_are_tolerated() {
    // Hand-craft a message with an unknown key besides `added`.
    // Bencoded dict keys must be in lexicographic order, so the unknown key
    // `xtra` sorts after `added.f`.
    use bencode::{ByteBuf, bencode_serialize_to_writer};
    use serde::Serialize;

    #[derive(Serialize)]
    struct Extended<'a> {
        added: Vec<ByteBuf<'a>>,
        #[serde(rename = "added.f")]
        added_f: ByteBuf<'a>,
        xtra: i64,
    }

    let mut buf = Vec::new();
    bencode_serialize_to_writer(
        &Extended {
            added: vec![ByteBuf(b"http://a/announce")],
            added_f: ByteBuf(&[FLAG_VERIFIED]),
            xtra: 42,
        },
        &mut buf,
    )
    .unwrap();
    let decoded = decode(&buf).unwrap();
    assert_eq!(decoded.len(), 1);
}

#[test]
fn iter_yields_zero_flag_when_no_flags_field() {
    let msg = LtTex::from_trackers(["http://a/announce", "http://b/announce"], None);
    let pairs: Vec<_> = msg.iter().collect();
    assert_eq!(pairs.len(), 2);
    assert_eq!(pairs[0].1, 0);
    assert_eq!(pairs[1].1, 0);
}

#[test]
fn flags_shorter_than_added_rejected() {
    // Even handcrafted bytes with truncated flags should be rejected at decode-time.
    use bencode::{ByteBuf, bencode_serialize_to_writer};
    use serde::Serialize;

    #[derive(Serialize)]
    struct Raw<'a> {
        added: Vec<ByteBuf<'a>>,
        #[serde(rename = "added.f")]
        added_f: ByteBuf<'a>,
    }

    let mut buf = Vec::new();
    bencode_serialize_to_writer(
        &Raw {
            added: vec![
                ByteBuf(b"http://a/announce"),
                ByteBuf(b"http://b/announce"),
                ByteBuf(b"http://c/announce"),
            ],
            added_f: ByteBuf(&[1, 0]),
        },
        &mut buf,
    )
    .unwrap();
    let err = decode(&buf).unwrap_err();
    assert!(matches!(
        err,
        Bep28Error::FlagsLengthMismatch {
            added_len: 3,
            flags_len: 2
        }
    ));
}
