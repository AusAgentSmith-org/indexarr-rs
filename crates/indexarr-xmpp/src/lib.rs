//! XMPP/Jabber channel — MUC-based peer discovery for indexarr.
//!
//! Mirrors the Python implementation in
//! `Indexarr/indexarr/sync/channels/xmpp.py`:
//!
//! - Joins a well-known MUC room.
//! - Node nicks follow `{contributor_id}|{external_http_url}`.
//! - Each new MUC occupant is validated with `GET /api/v1/sync/manifest`
//!   and, if responsive, inserted into the shared `PeerTable` with
//!   `source = "xmpp"`.
//!
//! We go through `tokio-xmpp` directly (rather than the higher-level
//! `xmpp` crate) because `xmpp` 0.6.0 drops non-self presence stanzas
//! and we need them to see who else is in the MUC.
//!
//! Auth uses XEP-0077 in-band registration on first run (derives JID +
//! password deterministically from the contributor id). Accounts that
//! already exist are reused.

mod register;

use std::collections::HashSet;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use futures::stream::StreamExt;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

use indexarr_core::config::Settings;
use indexarr_identity::ContributorIdentity;
use indexarr_sync::discovery::PeerTable;

use tokio_xmpp::AsyncClient as XmppClient;
use tokio_xmpp::AsyncConfig;
use tokio_xmpp::minidom;
use tokio_xmpp::parsers::jid::{BareJid, Jid};
use tokio_xmpp::parsers::muc::Muc;
use tokio_xmpp::parsers::presence::{Presence, Type as PresenceType};
use tokio_xmpp::starttls::ServerConfig;

pub struct XmppChannel {
    settings: Settings,
    identity: Arc<tokio::sync::RwLock<ContributorIdentity>>,
    peer_table: Arc<RwLock<PeerTable>>,
    http: reqwest::Client,
}

impl XmppChannel {
    pub fn new(
        settings: Settings,
        identity: Arc<tokio::sync::RwLock<ContributorIdentity>>,
        peer_table: Arc<RwLock<PeerTable>>,
    ) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .danger_accept_invalid_certs(!settings.sync_verify_tls)
            .build()
            .unwrap_or_default();
        Self {
            settings,
            identity,
            peer_table,
            http,
        }
    }

    /// Run the channel until the cancellation token fires.
    ///
    /// `tokio-xmpp`'s `AsyncClient` already handles reconnect internally
    /// when `set_reconnect(true)` is set, but we still wrap it in an
    /// outer loop so a fatal stream error (e.g. auth failure after a
    /// credential change) gets backed off rather than spun.
    pub async fn run(self, cancel: CancellationToken) {
        if !self.settings.xmpp_enabled {
            tracing::debug!("xmpp channel disabled");
            return;
        }

        let mut backoff = Duration::from_secs(5);
        let max_backoff = Duration::from_secs(300);

        loop {
            if cancel.is_cancelled() {
                break;
            }

            match self.run_once(cancel.clone()).await {
                Ok(()) => break, // clean shutdown
                Err(e) => tracing::warn!(error = %e, "xmpp channel error; backing off"),
            }

            tokio::select! {
                _ = tokio::time::sleep(backoff) => {}
                _ = cancel.cancelled() => break,
            }
            backoff = std::cmp::min(backoff * 2, max_backoff);
        }

        tracing::info!("xmpp channel stopped");
    }

    async fn run_once(&self, cancel: CancellationToken) -> Result<(), String> {
        // 1. Derive credentials from contributor id (same scheme as Python).
        let contributor_id = {
            let id = self.identity.read().await;
            match id.contributor_id() {
                Some(c) => c.to_string(),
                None => return Err("identity not initialized".to_string()),
            }
        };

        let (jid_str, password, muc_room, external_url) =
            self.build_credentials(&contributor_id)?;

        tracing::info!(jid = %jid_str, room = %muc_room, "xmpp: connecting");

        // 2. XEP-0077 in-band registration (no-op if the account exists).
        register::register_account(&jid_str, &password, &self.settings.xmpp_server).await;

        // 3. Connect and drive the event loop.
        let jid = BareJid::from_str(&jid_str).map_err(|e| format!("invalid JID: {e}"))?;
        let room: BareJid =
            BareJid::from_str(&muc_room).map_err(|e| format!("invalid MUC room: {e}"))?;
        let nick = format!("{contributor_id}|{external_url}");

        // Honour INDEXARR_XMPP_SERVER if set (host or host:port); otherwise
        // fall back to SRV lookup / the JID's domain. Matches Python's
        // `_parse_server` behaviour.
        let server = if self.settings.xmpp_server.is_empty() {
            ServerConfig::UseSrv
        } else {
            let (h, p) = match self.settings.xmpp_server.rsplit_once(':') {
                Some((h, p)) => (h.to_string(), p.parse::<u16>().unwrap_or(5222)),
                None => (self.settings.xmpp_server.clone(), 5222),
            };
            ServerConfig::Manual { host: h, port: p }
        };

        let mut client = XmppClient::new_with_config(AsyncConfig {
            jid: jid.into(),
            password,
            server,
        });
        client.set_reconnect(true);

        let mut seen: HashSet<String> = HashSet::new();

        loop {
            tokio::select! {
                _ = cancel.cancelled() => {
                    // Best-effort close.
                    let _ = client.send_end().await;
                    return Ok(());
                }
                next = client.next() => {
                    let Some(event) = next else {
                        return Err("xmpp stream ended".to_string());
                    };
                    if event.is_online() {
                        tracing::info!(room = %room, nick = %nick, "xmpp: online, joining MUC");
                        let room_with_nick = format!("{room}/{nick}");
                        let to: Jid = match Jid::from_str(&room_with_nick) {
                            Ok(j) => j,
                            Err(e) => return Err(format!("bad room/nick: {e}")),
                        };
                        let mut presence = Presence::new(PresenceType::None).with_to(to);
                        presence.add_payload(Muc::new());
                        let stanza: minidom::Element = presence.into();
                        if let Err(e) = client.send_stanza(stanza).await {
                            return Err(format!("muc join send_stanza: {e}"));
                        }
                    } else if let Some(element) = event.into_stanza()
                        && element.name() == "presence"
                        && let Err(e) = self.handle_presence(&element, &room, &nick, &mut seen).await
                    {
                        tracing::debug!(error = %e, "xmpp: presence handle error");
                    }
                }
            }
        }
    }

    /// A MUC presence stanza has `from = room@server/nick`. Extract the
    /// resource part as the occupant nick, parse it, and if it's new,
    /// validate the advertised HTTP URL and insert into PeerTable.
    async fn handle_presence(
        &self,
        element: &minidom::Element,
        room: &BareJid,
        our_nick: &str,
        seen: &mut HashSet<String>,
    ) -> Result<(), String> {
        // Try the strongly-typed parse first for correctness (ignores
        // failures — e.g. type="error" is still a valid XML presence
        // but may have unexpected payloads).
        let presence = match Presence::try_from(element.clone()) {
            Ok(p) => p,
            Err(_) => return Ok(()),
        };

        // Ignore our own self-presence and "unavailable" notifications.
        if !matches!(presence.type_, PresenceType::None) {
            return Ok(());
        }

        let from_jid = match presence.from {
            Some(j) => j,
            None => return Ok(()),
        };
        let from = match from_jid.try_into_full() {
            Ok(f) => f,
            Err(_) => return Ok(()), // bare JID; not a MUC occupant presence
        };

        if from.to_bare() != *room {
            return Ok(());
        }

        let nick_part = from.resource().as_str().to_string();
        if nick_part == our_nick {
            return Ok(());
        }
        if !seen.insert(nick_part.clone()) {
            return Ok(());
        }

        let (contributor_id, url) = match parse_nick(&nick_part) {
            Some(v) => v,
            None => {
                tracing::debug!(nick = %nick_part, "xmpp: unparseable nick, skipping");
                return Ok(());
            }
        };

        // Ghost session guard: don't peer with other sessions of ourselves.
        let our_contributor = our_nick.split('|').next().unwrap_or("");
        if contributor_id == our_contributor {
            return Ok(());
        }

        if !self.validate_peer(&url).await {
            tracing::info!(peer = %contributor_id, url = %url, "xmpp: peer failed validation");
            return Ok(());
        }

        let mut pt = self.peer_table.write().await;
        pt.add_peer(&contributor_id, &url, "xmpp");
        tracing::info!(peer = %contributor_id, url = %url, "xmpp: discovered peer");
        Ok(())
    }

    async fn validate_peer(&self, url: &str) -> bool {
        let manifest_url = format!("{}/api/v1/sync/manifest", url.trim_end_matches('/'));
        match self.http.get(&manifest_url).send().await {
            Ok(r) => r.status().is_success(),
            Err(_) => false,
        }
    }

    /// Build JID, password, MUC room JID, and the HTTP URL to advertise.
    fn build_credentials(
        &self,
        contributor_id: &str,
    ) -> Result<(String, String, String, String), String> {
        let muc_room = if self.settings.xmpp_muc_room.is_empty() {
            "indexarr-sync@conference.indexarr.net".to_string()
        } else {
            self.settings.xmpp_muc_room.clone()
        };

        let domain = muc_room
            .split_once("@conference.")
            .map(|(_, d)| d.to_string())
            .unwrap_or_else(|| "indexarr.net".to_string());

        let jid = if self.settings.xmpp_jid.is_empty() {
            format!("{}@{}", contributor_id.to_lowercase(), domain)
        } else {
            self.settings.xmpp_jid.clone()
        };

        let password = if self.settings.xmpp_password.is_empty() {
            contributor_id.to_string()
        } else {
            self.settings.xmpp_password.clone()
        };

        let external_url = if self.settings.sync_external_url.is_empty() {
            format!("http://{}:{}", self.settings.host, self.settings.port)
        } else {
            self.settings
                .sync_external_url
                .trim_end_matches('/')
                .to_string()
        };

        Ok((jid, password, muc_room, external_url))
    }
}

/// Split a MUC nick `"<contributor_id>|<url>"` into `(contributor_id, url)`.
fn parse_nick(nick: &str) -> Option<(String, String)> {
    let (cid, url) = nick.split_once('|')?;
    let cid = cid.trim();
    let url = url.trim().trim_end_matches('/');
    if cid.is_empty() || url.is_empty() {
        return None;
    }
    Some((cid.to_string(), url.to_string()))
}

#[cfg(test)]
mod tests {
    use super::parse_nick;

    #[test]
    fn parses_valid_nick() {
        let (c, u) = parse_nick("TN-abcd1234|http://example.com:8080/").unwrap();
        assert_eq!(c, "TN-abcd1234");
        assert_eq!(u, "http://example.com:8080");
    }

    #[test]
    fn rejects_no_separator() {
        assert!(parse_nick("TN-abcd1234").is_none());
    }

    #[test]
    fn rejects_empty_halves() {
        assert!(parse_nick("|http://x").is_none());
        assert!(parse_nick("TN-x|").is_none());
    }
}
