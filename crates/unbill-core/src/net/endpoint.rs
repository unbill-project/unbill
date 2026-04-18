// Iroh endpoint lifecycle and connection dispatch.
//
// `UnbillEndpoint` wraps `iroh::Endpoint`, opens it with the device secret key
// using the N0 preset (pkarr publishing + DNS lookup + default relay servers),
// and exposes the two runtime modes:
//   - `sync_once_inner`   — dial one peer and sync.
//   - `accept_loop_inner` — wait for incoming connections; dispatch by ALPN.

use std::sync::Arc;

use tracing::{info, warn};

use crate::model::NodeId;
use crate::service::UnbillService;

use super::identity::{run_identity_host, run_identity_requester};
use super::join::{run_join_host, run_join_requester};
use super::protocol::{JoinRequest, ALPN_IDENTITY, ALPN_JOIN, ALPN_SYNC};
use super::sync::run_sync_session;

pub struct UnbillEndpoint {
    inner: iroh::Endpoint,
}

impl UnbillEndpoint {
    /// Bind a new Iroh endpoint using the given device secret key.
    /// Uses the N0 preset: pkarr publishing + DNS address lookup + relay servers.
    pub async fn bind(secret_key: iroh::SecretKey) -> anyhow::Result<Self> {
        let inner = iroh::Endpoint::builder(iroh::endpoint::presets::N0)
            .secret_key(secret_key)
            .alpns(vec![
                ALPN_SYNC.to_vec(),
                ALPN_JOIN.to_vec(),
                ALPN_IDENTITY.to_vec(),
            ])
            .bind()
            .await?;
        Ok(Self { inner })
    }

    /// This device's `NodeId` as known to the network.
    pub fn node_id(&self) -> NodeId {
        NodeId::from_node_id(self.inner.id())
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

    pub(crate) async fn sync_once_inner(
        &self,
        peer: NodeId,
        svc: &UnbillService,
    ) -> anyhow::Result<()> {
        let addr = iroh::EndpointAddr::new(peer.as_node_id());
        let conn = self.inner.connect(addr, ALPN_SYNC).await?;
        let peer_node_id = NodeId::from_node_id(conn.remote_id());
        let (send, recv) = conn.open_bi().await?;
        run_sync_session(
            true,
            peer_node_id,
            &svc.ledgers,
            &svc.store,
            &svc.events,
            recv,
            send,
        )
        .await?;
        conn.close(0u32.into(), b"done");
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Initiator: join a ledger
    // -----------------------------------------------------------------------

    pub(crate) async fn join_ledger_inner(
        &self,
        host: NodeId,
        request: JoinRequest,
        svc: &UnbillService,
    ) -> anyhow::Result<()> {
        let addr = iroh::EndpointAddr::new(host.as_node_id());
        let conn = self.inner.connect(addr, ALPN_JOIN).await?;
        let (send, recv) = conn.open_bi().await?;
        run_join_requester(request, &svc.ledgers, &svc.store, &svc.events, recv, send).await?;
        conn.close(0u32.into(), b"done");
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Initiator: import identity
    // -----------------------------------------------------------------------

    pub(crate) async fn import_identity_inner(
        &self,
        host: NodeId,
        token: String,
        svc: &UnbillService,
    ) -> anyhow::Result<()> {
        let addr = iroh::EndpointAddr::new(host.as_node_id());
        let conn = self.inner.connect(addr, ALPN_IDENTITY).await?;
        let (send, recv) = conn.open_bi().await?;
        run_identity_requester(token, &svc.store, recv, send).await?;
        conn.close(0u32.into(), b"done");
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Responder: accept loop
    // -----------------------------------------------------------------------

    pub(crate) async fn accept_loop_inner(&self, svc: Arc<UnbillService>) -> anyhow::Result<()> {
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

            let peer = NodeId::from_node_id(conn.remote_id());

            let svc = Arc::clone(&svc);

            tokio::spawn(async move {
                if let Err(e) = dispatch(conn, peer, &alpn, svc).await {
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
    svc: Arc<UnbillService>,
) -> anyhow::Result<()> {
    match alpn {
        ALPN_SYNC => {
            let (send, recv) = conn.accept_bi().await?;
            run_sync_session(
                false,
                peer,
                &svc.ledgers,
                &svc.store,
                &svc.events,
                recv,
                send,
            )
            .await?;
        }
        ALPN_JOIN => {
            let (send, recv) = conn.accept_bi().await?;
            run_join_host(
                peer,
                &svc.pending_invitations,
                &svc.ledgers,
                &svc.store,
                &svc.events,
                recv,
                send,
            )
            .await?;
        }
        ALPN_IDENTITY => {
            let (send, recv) = conn.accept_bi().await?;
            run_identity_host(&svc.pending_identity_tokens, recv, send).await?;
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
