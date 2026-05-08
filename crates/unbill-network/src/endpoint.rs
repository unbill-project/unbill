// Iroh endpoint lifecycle and connection dispatch.
//
// `UnbillEndpoint` wraps `iroh::Endpoint`, opens it with the device secret key
// using the N0 preset (pkarr publishing + DNS lookup + default relay servers),
// and exposes the two runtime modes:
//   - `sync_once_inner`   — dial one peer and sync.
//   - `accept_loop_inner` — wait for incoming connections; dispatch by ALPN.

use std::sync::Arc;

use tokio::sync::broadcast;
use tracing::{info, warn};

use unbill_event::ServiceEvent;
use unbill_model::{NodeId, SecretKey};

use crate::node_id_ext::SecretKeyExt;
use unbill_storage::LedgerStore;

use crate::join::{run_join_host, run_join_requester};
use crate::node_id_ext::{EndpointIdExt, NodeIdExt};
use crate::protocol::{ALPN_JOIN, ALPN_SYNC, JoinRequest};
use crate::sync::run_sync_session;

pub struct UnbillEndpoint {
    inner: iroh::Endpoint,
}

impl UnbillEndpoint {
    /// Bind a new Iroh endpoint using the given device secret key.
    /// Uses the N0 preset: pkarr publishing + DNS address lookup + relay servers.
    pub async fn bind(key: &SecretKey) -> anyhow::Result<Self> {
        let inner = iroh::Endpoint::builder(iroh::endpoint::presets::N0)
            .secret_key(key.to_iroh_key())
            .alpns(vec![ALPN_SYNC.to_vec(), ALPN_JOIN.to_vec()])
            .bind()
            .await?;
        Ok(Self { inner })
    }

    /// This device's `NodeId` as known to the network.
    pub fn node_id(&self) -> NodeId {
        self.inner.id().to_node_id()
    }

    /// The endpoint's current relay URL, if Iroh has selected one.
    pub fn relay_url(&self) -> Option<String> {
        self.inner
            .addr()
            .relay_urls()
            .next()
            .map(ToString::to_string)
    }

    /// Wait until the endpoint has a relay connection — the relay is the
    /// reliable path that enables connectivity before direct addresses are
    /// established via hole-punching.
    pub async fn wait_for_ready(&self) {
        self.inner.online().await;
    }

    /// Close the endpoint gracefully.
    pub async fn close(self) {
        self.inner.close().await;
    }

    // -----------------------------------------------------------------------
    // Initiator: sync once
    // -----------------------------------------------------------------------

    pub async fn sync_once_inner(
        &self,
        peer: NodeId,
        store: &Arc<dyn LedgerStore>,
        events: &broadcast::Sender<ServiceEvent>,
    ) -> anyhow::Result<()> {
        let addr = iroh::EndpointAddr::new(peer.to_endpoint_id()?);
        self.sync_once_addr(addr, store, events).await
    }

    pub async fn sync_once_with_relay_inner(
        &self,
        peer: NodeId,
        relay_url: &str,
        store: &Arc<dyn LedgerStore>,
        events: &broadcast::Sender<ServiceEvent>,
    ) -> anyhow::Result<()> {
        let addr =
            iroh::EndpointAddr::new(peer.to_endpoint_id()?).with_relay_url(relay_url.parse()?);
        self.sync_once_addr(addr, store, events).await
    }

    async fn sync_once_addr(
        &self,
        addr: iroh::EndpointAddr,
        store: &Arc<dyn LedgerStore>,
        events: &broadcast::Sender<ServiceEvent>,
    ) -> anyhow::Result<()> {
        let conn = self.inner.connect(addr, ALPN_SYNC).await?;
        let peer_node_id = conn.remote_id().to_node_id();
        let (send, recv) = conn.open_bi().await?;
        run_sync_session(true, peer_node_id, store, events, recv, send).await?;
        conn.close(0u32.into(), b"done");
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Initiator: join a ledger
    // -----------------------------------------------------------------------

    pub async fn join_ledger_inner(
        &self,
        host: NodeId,
        local_label: Option<String>,
        request: JoinRequest,
        store: &Arc<dyn LedgerStore>,
        events: &broadcast::Sender<ServiceEvent>,
    ) -> anyhow::Result<()> {
        let addr = iroh::EndpointAddr::new(host.to_endpoint_id()?);
        self.join_ledger_addr(host, local_label, request, addr, store, events)
            .await
    }

    pub async fn join_ledger_with_relay_inner(
        &self,
        host: NodeId,
        relay_url: &str,
        local_label: Option<String>,
        request: JoinRequest,
        store: &Arc<dyn LedgerStore>,
        events: &broadcast::Sender<ServiceEvent>,
    ) -> anyhow::Result<()> {
        let addr =
            iroh::EndpointAddr::new(host.to_endpoint_id()?).with_relay_url(relay_url.parse()?);
        self.join_ledger_addr(host, local_label, request, addr, store, events)
            .await
    }

    async fn join_ledger_addr(
        &self,
        host: NodeId,
        local_label: Option<String>,
        request: JoinRequest,
        addr: iroh::EndpointAddr,
        store: &Arc<dyn LedgerStore>,
        events: &broadcast::Sender<ServiceEvent>,
    ) -> anyhow::Result<()> {
        let conn = self.inner.connect(addr, ALPN_JOIN).await?;
        let (send, recv) = conn.open_bi().await?;
        run_join_requester(host, local_label, request, store, events, recv, send).await?;
        conn.close(0u32.into(), b"done");
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Responder: accept loop
    // -----------------------------------------------------------------------

    pub async fn accept_loop_inner(
        &self,
        store: Arc<dyn LedgerStore>,
        events: broadcast::Sender<ServiceEvent>,
    ) -> anyhow::Result<()> {
        loop {
            let incoming = match self.inner.accept().await {
                None => {
                    info!("endpoint closed, stopping accept loop");
                    break;
                }
                Some(inc) => inc,
            };

            let mut connecting = match incoming.accept() {
                Ok(c) => c,
                Err(e) => {
                    warn!("rejected incoming QUIC handshake: {e}");
                    continue;
                }
            };

            // Read ALPN before completing the handshake so we can dispatch.
            let alpn = match connecting.alpn().await {
                Ok(a) => a,
                Err(e) => {
                    warn!("could not read ALPN from incoming connection: {e}");
                    continue;
                }
            };

            let conn = match connecting.await {
                Ok(c) => c,
                Err(e) => {
                    warn!("incoming connection handshake failed: {e}");
                    continue;
                }
            };

            let peer = conn.remote_id().to_node_id();
            let store = Arc::clone(&store);
            let events = events.clone();

            tokio::spawn(async move {
                if let Err(e) = dispatch(conn, peer, &alpn, store, events).await {
                    warn!("connection handler error: {e:#}");
                }
            });
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Dispatch incoming connection to the right protocol handler
// ---------------------------------------------------------------------------

async fn dispatch(
    conn: iroh::endpoint::Connection,
    peer: NodeId,
    alpn: &[u8],
    store: Arc<dyn LedgerStore>,
    events: broadcast::Sender<ServiceEvent>,
) -> anyhow::Result<()> {
    match alpn {
        ALPN_SYNC => {
            let (send, recv) = conn.accept_bi().await?;
            run_sync_session(false, peer, &store, &events, recv, send).await?;
        }
        ALPN_JOIN => {
            let (send, recv) = conn.accept_bi().await?;
            run_join_host(peer, &store, &events, recv, send).await?;
        }
        other => {
            anyhow::bail!(
                "unknown ALPN from {peer}: {:?}",
                String::from_utf8_lossy(other)
            );
        }
    }
    // Wait for the initiator to close the connection.  The initiator calls
    // conn.close() only after it has finished reading, which guarantees all
    // stream data was delivered before we exit.
    conn.closed().await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relay_address_contains_peer_and_relay() {
        let peer = NodeId::from_seed(7);
        let relay_url = "https://relay.example.com/";

        let addr = iroh::EndpointAddr::new(peer.to_endpoint_id().unwrap())
            .with_relay_url(relay_url.parse().unwrap());

        assert_eq!(addr.id.to_node_id(), peer);
        assert_eq!(
            addr.relay_urls().next().map(ToString::to_string),
            Some(relay_url.to_string())
        );
    }
}
