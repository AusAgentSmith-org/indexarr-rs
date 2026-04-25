//! BEP 28 — Tracker Exchange (`lt_tex`).
//!
//! Implements the `lt_tex` extended message defined in
//! [BEP 28](https://www.bittorrent.org/beps/bep_0028.html). Lives in
//! `indexarr-rs` as a standalone codec while it stabilises; once upstreamed
//! into `librtbit-peer-protocol` 0.2.0 this crate is retired
//! (see `bep-uplift.md`).
//!
//! BEP 28 is a *net-new* protocol on both the Rust and Python sides — neither
//! `librtbit-peer-protocol` nor `btpydht` carries any prior art. The shape
//! mirrors `librtbit-peer-protocol::extended::ut_pex` for consistency.
//!
//! # Wire format
//!
//! The message payload is a bencoded dict:
//! ```text
//! {
//!     "added": [<tracker URL bytes>, ...],     // required if non-empty
//!     "added.f": <bytes>,                      // optional, one flag byte per tracker
//! }
//! ```
//!
//! Per the spec, peers SHOULD NOT send `lt_tex` more than once per minute.
//! That throttling is the caller's responsibility — this crate is codec-only.

use bencode::{ByteBufOwned, bencode_serialize_to_writer, from_bytes};
use serde::{Deserialize, Serialize};

/// Method name registered in the BEP 10 extended-handshake `m` dict for this
/// extension. Caller advertises this string with their chosen u8 message id.
pub const METHOD_NAME: &[u8] = b"lt_tex";

/// Spec-recommended minimum interval between outgoing lt_tex messages from
/// the same peer (per BEP 28: "no more often than once per minute").
pub const MIN_SEND_INTERVAL_SECS: u64 = 60;

/// Flag bit indicating the tracker is "verified" — i.e. the sender has
/// successfully scraped/announced against it. Bit 0 of each flag byte.
pub const FLAG_VERIFIED: u8 = 0x01;

/// `lt_tex` message payload.
///
/// `added` carries the tracker URL list. `added_f` (`added.f` on the wire)
/// optionally carries one flag byte per tracker, in the same order. If
/// present, `added_f.len()` MUST equal `added.len()` — see
/// [`validate`](Self::validate).
#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LtTex {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub added: Option<Vec<ByteBufOwned>>,
    #[serde(rename = "added.f")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub added_f: Option<ByteBufOwned>,
}

/// Errors raised by the BEP 28 codec.
#[derive(Debug, thiserror::Error)]
pub enum Bep28Error {
    #[error("bencode serialize: {0}")]
    Serialize(#[from] bencode::SerializeError),
    #[error("bencode deserialize: {0}")]
    Deserialize(String),
    #[error("added.f length ({flags_len}) does not match added length ({added_len})")]
    FlagsLengthMismatch { added_len: usize, flags_len: usize },
}

impl LtTex {
    /// Build an `LtTex` from a list of tracker URLs and optional flags. Flags,
    /// if provided, must align 1:1 with the URL list — caller is responsible.
    pub fn from_trackers<I, S>(trackers: I, flags: Option<Vec<u8>>) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<[u8]>,
    {
        let added: Vec<ByteBufOwned> = trackers
            .into_iter()
            .map(|s| ByteBufOwned::from(s.as_ref().to_vec()))
            .collect();
        let added = if added.is_empty() { None } else { Some(added) };
        let added_f = flags.map(ByteBufOwned::from);
        Self { added, added_f }
    }

    /// Iterate over `(tracker_url_bytes, flags)` pairs. If no flags are
    /// present the second element is 0 for every tracker.
    pub fn iter(&self) -> impl Iterator<Item = (&[u8], u8)> {
        let added = self.added.iter().flat_map(|v| v.iter());
        let flags = self.added_f.as_ref().map(|f| f.as_ref());
        added.enumerate().map(move |(i, t)| {
            let flag = flags.and_then(|f| f.get(i).copied()).unwrap_or(0);
            (t.as_ref(), flag)
        })
    }

    /// Number of trackers advertised in the message.
    pub fn len(&self) -> usize {
        self.added.as_ref().map(|v| v.len()).unwrap_or(0)
    }

    /// Whether the message is empty (no trackers advertised).
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Validate that, when `added.f` is present, its length matches `added`.
    /// Per BEP 28 the flag list pairs 1:1 with the URL list.
    pub fn validate(&self) -> Result<(), Bep28Error> {
        match (&self.added, &self.added_f) {
            (Some(added), Some(flags)) => {
                if added.len() == flags.as_ref().len() {
                    Ok(())
                } else {
                    Err(Bep28Error::FlagsLengthMismatch {
                        added_len: added.len(),
                        flags_len: flags.as_ref().len(),
                    })
                }
            }
            (None, Some(flags)) if !flags.as_ref().is_empty() => {
                Err(Bep28Error::FlagsLengthMismatch {
                    added_len: 0,
                    flags_len: flags.as_ref().len(),
                })
            }
            _ => Ok(()),
        }
    }
}

/// Encode an `LtTex` message to bencode. Validates flag alignment first.
pub fn encode(msg: &LtTex) -> Result<Vec<u8>, Bep28Error> {
    msg.validate()?;
    let mut buf = Vec::new();
    bencode_serialize_to_writer(msg, &mut buf)?;
    Ok(buf)
}

/// Decode an `LtTex` message from bencode bytes. Validates flag alignment.
pub fn decode(buf: &[u8]) -> Result<LtTex, Bep28Error> {
    let msg: LtTex = from_bytes(buf).map_err(|e| Bep28Error::Deserialize(format!("{e}")))?;
    msg.validate()?;
    Ok(msg)
}
