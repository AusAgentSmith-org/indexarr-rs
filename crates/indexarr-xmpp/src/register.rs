//! XEP-0077 in-band registration over raw TCP.
//!
//! The `xmpp-rs` client doesn't expose registration, so we open a raw
//! stream and speak the protocol directly — same approach as the Python
//! reference (`xmpp.py::_register_account`). We don't negotiate TLS:
//! this only runs once on first boot to provision the shared account
//! for this node, and the password is a deterministic derivation of the
//! contributor id (not a secret worth protecting from an on-path
//! attacker — the sync protocol itself is the authority).

use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

/// Parse `host:port` → `(host, port)` with a default XMPP port.
fn parse_server(server: &str) -> Option<(String, u16)> {
    if server.is_empty() {
        return None;
    }
    let (host, port) = match server.split_once(':') {
        Some((h, p)) => (h.to_string(), p.parse::<u16>().unwrap_or(5222)),
        None => (server.to_string(), 5222),
    };
    Some((host, port))
}

/// Attempt XEP-0077 in-band registration. Safe to call on every startup;
/// a 409/conflict from the server means the account already exists,
/// which we treat as success.
///
/// Note: on failure we only log and return — the subsequent auth
/// attempt will then tell us clearly whether the account is usable.
pub async fn register_account(jid: &str, password: &str, server_cfg: &str) {
    let (username, domain) = match jid.split_once('@') {
        Some((u, d)) => (u.to_string(), d.to_string()),
        None => {
            tracing::warn!(jid, "xmpp register: invalid JID, skipping");
            return;
        }
    };

    let (host, port) = parse_server(server_cfg).unwrap_or_else(|| (domain.clone(), 5222));

    let addr = format!("{host}:{port}");
    let stream_result =
        tokio::time::timeout(Duration::from_secs(10), TcpStream::connect(&addr)).await;

    let mut stream = match stream_result {
        Ok(Ok(s)) => s,
        Ok(Err(e)) => {
            tracing::warn!(error = %e, addr, "xmpp register: connect failed");
            return;
        }
        Err(_) => {
            tracing::warn!(addr, "xmpp register: connect timed out");
            return;
        }
    };

    // Send stream header.
    let open = format!(
        "<?xml version=\"1.0\"?>\
         <stream:stream xmlns=\"jabber:client\" \
         xmlns:stream=\"http://etherx.jabber.org/streams\" \
         to=\"{domain}\" version=\"1.0\">"
    );
    if let Err(e) = stream.write_all(open.as_bytes()).await {
        tracing::warn!(error = %e, "xmpp register: write stream header failed");
        return;
    }

    // Drain the stream-features response (we don't parse — we just need
    // the socket ready for the registration IQ).
    let mut buf = [0u8; 4096];
    let _ = tokio::time::timeout(Duration::from_secs(3), stream.read(&mut buf)).await;

    let iq = format!(
        "<iq type=\"set\" id=\"reg1\">\
         <query xmlns=\"jabber:iq:register\">\
         <username>{username}</username>\
         <password>{password}</password>\
         </query></iq>"
    );
    if let Err(e) = stream.write_all(iq.as_bytes()).await {
        tracing::warn!(error = %e, "xmpp register: write IQ failed");
        return;
    }

    let resp = match tokio::time::timeout(Duration::from_secs(5), stream.read(&mut buf)).await {
        Ok(Ok(n)) if n > 0 => String::from_utf8_lossy(&buf[..n]).to_string(),
        _ => {
            tracing::debug!("xmpp register: no response (account likely exists)");
            return;
        }
    };

    let _ = stream.shutdown().await;

    if resp.contains(r#"type="result""#) {
        tracing::info!(user = %username, "xmpp register: account registered");
    } else if resp.contains("conflict") || resp.contains("409") {
        tracing::debug!(user = %username, "xmpp register: account already exists");
    } else {
        let short: String = resp.chars().take(200).collect();
        tracing::warn!(user = %username, response = %short, "xmpp register: unexpected response");
    }
}
