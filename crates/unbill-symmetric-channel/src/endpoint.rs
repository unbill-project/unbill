// Iroh endpoint lifecycle and connection dispatch.
//
// `UnbillEndpoint` wraps `iroh::Endpoint`, opens it with the device secret key
// and exposes the two runtime modes:
//   - `sync_once_inner`   — dial one peer and sync.
//   - `accept_loop_inner` — wait for incoming connections; dispatch by ALPN.
//
// Connectivity strategy:
//   - All endpoints share a single well-known relay (UNBILL_RELAY_URL).
//     This means any peer's `EndpointAddr` can be constructed from their
//     NodeId alone — no pkarr/DNS lookup required when dialing.
//   - mDNS discovers peers on the same LAN so that two devices can connect
//     directly even when the relay is unreachable (e.g. no internet).
//   - A `MemoryLookup` caches direct addresses learned from previous
//     connections to reduce relay traffic after hole-punching succeeds.
//   - pkarr publishing is kept so interop with other iroh clients works.

use std::sync::Arc;

use tracing::{info, warn};

use unbill_model::{NodeId, SecretKey, UnbillError};

type Result<T> = std::result::Result<T, UnbillError>;

use crate::node_id_ext::SecretKeyExt;
use unbill_storage::LedgerStore;

use iroh::RelayMode;
use iroh::address_lookup::MdnsAddressLookup;
use iroh::address_lookup::memory::MemoryLookup;
use iroh::endpoint::presets;

use crate::join::{run_join_host, run_join_requester};
use crate::node_id_ext::{EndpointIdExt, NodeIdExt};
use crate::protocol::{ALPN_JOIN, ALPN_SYNC, JoinRequest};
use crate::sync::run_sync_session;

// sirno:witness:security-model:begin
/// Relays shared by all unbill endpoints.
/// All peers register with one of these relays, so any peer's `EndpointAddr`
/// can be constructed from their `NodeId` alone — no DNS/pkarr lookup required.
const UNBILL_RELAY_URLS: &[&str] = &[
    "https://use1-1.relay.n0.iroh-canary.iroh.link.",
    "https://usw1-1.relay.n0.iroh-canary.iroh.link.",
    "https://euc1-1.relay.n0.iroh-canary.iroh.link.",
    "https://aps1-1.relay.n0.iroh-canary.iroh.link.",
];
// sirno:witness:security-model:end

// sirno:witness:symmetric-channel:begin
pub struct UnbillEndpoint {
    inner: iroh::Endpoint,
    addr_cache: MemoryLookup,
}

impl UnbillEndpoint {
    /// Bind a new Iroh endpoint using the given device secret key.
    pub async fn bind(key: &SecretKey) -> Result<Self> {
        let mut relay_urls: Vec<iroh::RelayUrl> = Vec::new();
        for s in UNBILL_RELAY_URLS {
            relay_urls.push(
                s.parse::<iroh::RelayUrl>()
                    .map_err(|e| UnbillError::Network(format!("relay URL parse: {e}")))?,
            );
        }
        let addr_cache = MemoryLookup::new();
        let inner = iroh::Endpoint::builder(presets::Minimal)
            .secret_key(key.to_iroh_key())
            .alpns(vec![ALPN_SYNC.to_vec(), ALPN_JOIN.to_vec()])
            .relay_mode(RelayMode::custom(relay_urls))
            .address_lookup(iroh::address_lookup::PkarrPublisher::n0_dns())
            .address_lookup(MdnsAddressLookup::builder())
            .address_lookup(addr_cache.clone())
            .bind()
            .await
            .map_err(|e| UnbillError::Network(e.to_string()))?;
        Ok(Self { inner, addr_cache })
    }

    /// Cache the remote peer's direct addresses after a successful connection
    /// so that future dials can use the direct path instead of the relay.
    async fn cache_peer(&self, conn: &iroh::endpoint::Connection) {
        let id = conn.remote_id();
        if let Some(info) = self.inner.remote_info(id).await {
            let addr =
                iroh::EndpointAddr::from_parts(info.id(), info.into_addrs().map(|a| a.into_addr()));
            self.addr_cache.add_endpoint_info(addr);
        }
    }

    /// Build the `EndpointAddr` for dialing a peer.
    /// Since all unbill endpoints register with `UNBILL_RELAY_URL`, the relay
    /// path is always known — no lookup required.
    fn peer_addr(&self, peer: &NodeId) -> Result<iroh::EndpointAddr> {
        let mut addr = iroh::EndpointAddr::new(
            peer.to_endpoint_id()
                .map_err(|e| UnbillError::Network(e.to_string()))?,
        );
        for url in UNBILL_RELAY_URLS {
            addr = addr.with_relay_url(
                url.parse::<iroh::RelayUrl>()
                    .map_err(|e| UnbillError::Network(format!("relay URL parse: {e}")))?,
            );
        }
        Ok(addr)
    }

    /// This device's `NodeId` as known to the network.
    pub fn node_id(&self) -> NodeId {
        self.inner.id().to_node_id()
    }

    /// Wait until the endpoint has a relay connection.
    pub async fn wait_for_ready(&self) {
        self.inner.online().await;
    }

    /// Close the endpoint gracefully.
    pub async fn close(&self) {
        self.inner.close().await;
    }

    // -----------------------------------------------------------------------
    // Initiator: sync once
    // -----------------------------------------------------------------------

    pub async fn sync_once_inner(&self, peer: NodeId, store: &Arc<dyn LedgerStore>) -> Result<()> {
        let conn = self
            .inner
            .connect(self.peer_addr(&peer)?, ALPN_SYNC)
            .await
            .map_err(|e| UnbillError::Network(e.to_string()))?;
        self.cache_peer(&conn).await;
        let peer_node_id = conn.remote_id().to_node_id();
        let (send, recv) = conn
            .open_bi()
            .await
            .map_err(|e| UnbillError::Network(e.to_string()))?;
        run_sync_session(true, peer_node_id, store, recv, send).await?;
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
    ) -> Result<()> {
        let conn = self
            .inner
            .connect(self.peer_addr(&host)?, ALPN_JOIN)
            .await
            .map_err(|e| UnbillError::Network(e.to_string()))?;
        self.cache_peer(&conn).await;
        let (send, recv) = conn
            .open_bi()
            .await
            .map_err(|e| UnbillError::Network(e.to_string()))?;
        run_join_requester(host, local_label, request, store, recv, send).await?;
        conn.close(0u32.into(), b"done");
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Responder: accept loop
    // -----------------------------------------------------------------------

    pub async fn accept_loop_inner(&self, store: Arc<dyn LedgerStore>) -> Result<()> {
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

            self.cache_peer(&conn).await;
            let peer = conn.remote_id().to_node_id();
            let store = Arc::clone(&store);

            tokio::spawn(async move {
                if let Err(e) = dispatch(conn, peer, &alpn, store).await {
                    warn!("connection handler error: {e:#}");
                }
            });
        }
        Ok(())
    }
}
// sirno:witness:symmetric-channel:end

// ---------------------------------------------------------------------------
// Dispatch incoming connection to the right protocol handler
// ---------------------------------------------------------------------------

async fn dispatch(
    conn: iroh::endpoint::Connection,
    peer: NodeId,
    alpn: &[u8],
    store: Arc<dyn LedgerStore>,
) -> Result<()> {
    match alpn {
        ALPN_SYNC => {
            let (send, recv) = conn
                .accept_bi()
                .await
                .map_err(|e| UnbillError::Network(e.to_string()))?;
            run_sync_session(false, peer, &store, recv, send).await?;
        }
        ALPN_JOIN => {
            let (send, recv) = conn
                .accept_bi()
                .await
                .map_err(|e| UnbillError::Network(e.to_string()))?;
            run_join_host(peer, &store, recv, send).await?;
        }
        other => {
            return Err(UnbillError::Network(format!(
                "unknown ALPN from {peer}: {:?}",
                String::from_utf8_lossy(other)
            )));
        }
    }
    // Wait for the initiator to close the connection.  The initiator calls
    // conn.close() only after it has finished reading, which guarantees all
    // stream data was delivered before we exit.
    conn.closed().await;
    Ok(())
}
