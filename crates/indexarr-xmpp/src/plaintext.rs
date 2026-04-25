//! Plaintext `ServerConnector` for tokio-xmpp.
//!
//! `tokio-xmpp` 4.0's stock `ServerConfig` (the `starttls` module) is
//! hard-coded to require STARTTLS — if the server doesn't advertise it,
//! the connection fails. That's a reasonable default for the public
//! internet, but our XMPP server (Prosody on Vultr) is reached over
//! Tailscale or by explicit `host:port` config, with the password
//! derived deterministically from the contributor id (so confidentiality
//! of the SASL exchange isn't load-bearing).
//!
//! This connector just opens a TCP stream and starts the XMPP session
//! without any TLS upgrade. It's only used when `INDEXARR_XMPP_SERVER`
//! is set explicitly; the default path still goes through SRV+STARTTLS.

use std::io;

use tokio::net::TcpStream;
use tokio_xmpp::Error as XmppError;
use tokio_xmpp::connect::{ServerConnector, ServerConnectorError};
use tokio_xmpp::parsers::jid::Jid;
use tokio_xmpp::xmpp_stream::XMPPStream;

#[derive(Debug, Clone)]
pub struct PlaintextConnector {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, thiserror::Error)]
pub enum PlaintextError {
    #[error("io: {0}")]
    Io(#[from] io::Error),
    #[error("xmpp: {0}")]
    Xmpp(#[from] XmppError),
}

impl ServerConnectorError for PlaintextError {}

impl ServerConnector for PlaintextConnector {
    type Stream = TcpStream;
    type Error = PlaintextError;

    async fn connect(&self, jid: &Jid, ns: &str) -> Result<XMPPStream<Self::Stream>, Self::Error> {
        let tcp = TcpStream::connect((self.host.as_str(), self.port)).await?;
        let stream = XMPPStream::start(tcp, jid.clone(), ns.to_owned()).await?;
        Ok(stream)
    }
}
