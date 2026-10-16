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

use tokio::io::{AsyncRead, AsyncWrite};
use tracing::{info, warn};

use unbill_model::{NodeId, SecretKey, UnbillError};

type Result<T> = std::result::Result<T, UnbillError>;

use crate::node_id_ext::SecretKeyExt;
use unbill_storage::StoreServer;

use futures_util::StreamExt;
use iroh::RelayMode;
use iroh::address_lookup::memory::MemoryLookup;
use iroh::endpoint::presets;
use iroh_mdns_address_lookup::MdnsAddressLookup;

use crate::join::{run_join_host, run_join_requester};
use crate::node_id_ext::{EndpointIdExt, NodeIdExt};
use crate::protocol::{ALPN_JOIN, ALPN_SYNC, JoinRequest};
use crate::sync::run_sync_session;

// ---------------------------------------------------------------------------
// Connection wrappers (avoid leaking iroh types to dependents)
// ---------------------------------------------------------------------------

pub struct AcceptedBi {
    pub peer: NodeId,
    pub alpn: Vec<u8>,
    pub send: Box<dyn AsyncWrite + Unpin + Send>,
    pub recv: Box<dyn AsyncRead + Unpin + Send>,
    pub handle: ConnHandle,
}

/// Connection handle for waiting on graceful close.
pub struct ConnHandle {
    conn: iroh::endpoint::Connection,
}

impl ConnHandle {
    /// Wait for the remote side to close the connection.
    pub async fn wait_for_close(&self) {
        self.conn.closed().await;
    }

    /// Signal done and close the connection.
    pub fn close(&self) {
        self.conn.close(0u32.into(), b"done");
    }
}

pub struct ConnectedBi {
    pub send: Box<dyn AsyncWrite + Unpin + Send>,
    pub recv: Box<dyn AsyncRead + Unpin + Send>,
    pub handle: ConnHandle,
}

// sirno:witness:security-model:begin
/// Relays shared by all unbill endpoints.
/// All peers register with one of these relays, so any peer's `EndpointAddr`
/// can be constructed from their `NodeId` alone — no DNS/pkarr lookup required.
const UNBILL_RELAY_URLS: &[&str] = &[
    "https://use1-1.relay.n0.iroh.link.",
    "https://usw1-1.relay.n0.iroh.link.",
    "https://euc1-1.relay.n0.iroh.link.",
    "https://aps1-1.relay.n0.iroh.link.",
];
// sirno:witness:security-model:end

// sirno:witness:symmetric-channel:begin
pub struct UnbillEndpoint {
    inner: iroh::Endpoint,
    addr_cache: MemoryLookup,
    incoming_tasks: tokio::sync::Mutex<tokio::task::JoinSet<Option<AcceptedBi>>>,
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
        #[allow(unused_mut)]
        let mut builder = iroh::Endpoint::builder(presets::Minimal)
            .secret_key(key.to_iroh_key())
            .alpns(vec![ALPN_SYNC.to_vec(), ALPN_JOIN.to_vec()])
            .relay_mode(RelayMode::custom(relay_urls))
            .address_lookup(iroh::address_lookup::PkarrPublisher::n0_dns())
            .address_lookup(MdnsAddressLookup::builder())
            .address_lookup(addr_cache.clone());

        // On Android, reading the system DNS config panics because
        // ndk-context is not initialised by Tauri.  Use Google Public DNS
        // directly so hickory-resolver never touches ndk-context.
        #[cfg(target_os = "android")]
        {
            let dns = iroh::dns::DnsResolver::with_nameserver(([8, 8, 8, 8], 53).into());
            builder = builder.dns_resolver(dns);
        }

        let inner = builder
            .bind()
            .await
            .map_err(|e| UnbillError::Network(e.to_string()))?;
        Ok(Self {
            inner,
            addr_cache,
            incoming_tasks: tokio::sync::Mutex::new(tokio::task::JoinSet::new()),
        })
    }

    /// Cache the remote peer's direct addresses after a successful connection
    /// so that future dials can use the direct path instead of the relay.
    async fn cache_peer(&self, conn: &iroh::endpoint::Connection) {
        monitor_paths(conn);
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
    // Accept one incoming connection (returns None when endpoint closes)
    // -----------------------------------------------------------------------

    pub async fn accept_bi(&self) -> Option<AcceptedBi> {
        // Keep unfinished tasks across calls, including while callers handle a result.
        let mut tasks = self.incoming_tasks.lock().await;
        loop {
            tokio::select! {
                result = tasks.join_next(), if !tasks.is_empty() => {
                    match result {
                        Some(Ok(Some(accepted))) => {
                            self.cache_peer(&accepted.handle.conn).await;
                            return Some(accepted);
                        }
                        Some(Err(error)) => warn!("incoming connection task failed: {error}"),
                        _ => {}
                    }
                }
                incoming = self.inner.accept() => {
                    let Some(incoming) = incoming else {
                        tasks.abort_all();
                        return None;
                    };
                    tasks.spawn(accept_incoming(incoming));
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // Initiator: connect for join protocol
    // -----------------------------------------------------------------------

    pub async fn connect_bi_join(&self, host: NodeId) -> Result<ConnectedBi> {
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
        Ok(ConnectedBi {
            send: Box::new(send),
            recv: Box::new(recv),
            handle: ConnHandle { conn },
        })
    }

    // -----------------------------------------------------------------------
    // Initiator: connect for sync protocol
    // -----------------------------------------------------------------------

    pub async fn connect_bi_sync(&self, peer: NodeId) -> Result<ConnectedBi> {
        let conn = self
            .inner
            .connect(self.peer_addr(&peer)?, ALPN_SYNC)
            .await
            .map_err(|e| UnbillError::Network(e.to_string()))?;
        self.cache_peer(&conn).await;
        let (send, recv) = conn
            .open_bi()
            .await
            .map_err(|e| UnbillError::Network(e.to_string()))?;
        Ok(ConnectedBi {
            send: Box::new(send),
            recv: Box::new(recv),
            handle: ConnHandle { conn },
        })
    }

    // -----------------------------------------------------------------------
    // Initiator: sync once
    // -----------------------------------------------------------------------

    pub async fn sync_once_inner(&self, peer: NodeId, store: &StoreServer) -> Result<()> {
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
        request: JoinRequest,
        store: &StoreServer,
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
        run_join_requester(request, store, recv, send).await?;
        conn.close(0u32.into(), b"done");
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Responder: accept loop
    // -----------------------------------------------------------------------

    pub async fn accept_loop_inner(&self, store: Arc<StoreServer>) -> Result<()> {
        while let Some(accepted) = self.accept_bi().await {
            let store = Arc::clone(&store);
            tokio::spawn(async move {
                if let Err(e) = dispatch(accepted, store).await {
                    warn!("connection handler error: {e:#}");
                }
            });
        }
        info!("endpoint closed, stopping accept loop");
        Ok(())
    }
}
// sirno:witness:symmetric-channel:end

// ---------------------------------------------------------------------------
// Dispatch incoming connection to the right protocol handler
// ---------------------------------------------------------------------------

// Path events own their data; the observer never retains a strong connection.
fn monitor_paths(conn: &iroh::endpoint::Connection) {
    let connection = conn.stable_id();
    let peer = conn.remote_id();
    let protocol = String::from_utf8_lossy(conn.alpn()).into_owned();
    let weak = conn.weak_handle();
    let mut events = conn.path_events();
    log_paths(conn, "established");
    tokio::spawn(async move {
        while let Some(event) = events.next().await {
            if let iroh::endpoint::PathEvent::Closed {
                remote_addr,
                last_stats,
                ..
            } = &event
            {
                info!(connection, %peer, %protocol, event = "closed",
                    transport = transport_name(remote_addr), remote = %remote_addr,
                    selected = false, closed = true, sent_bytes = last_stats.udp_tx.bytes,
                    received_bytes = last_stats.udp_rx.bytes, "connection path");
            } else if let Some(conn) = weak.upgrade() {
                log_paths(&conn, "changed");
            }
        }
        info!(connection, %peer, %protocol, event = "closed", "connection closed");
    });
}

fn transport_name(address: &iroh::TransportAddr) -> &'static str {
    match address {
        iroh::TransportAddr::Ip(_) => "direct",
        iroh::TransportAddr::Relay(_) => "relay",
        _ => "custom",
    }
}

fn log_paths(conn: &iroh::endpoint::Connection, event: &str) {
    let connection = conn.stable_id();
    let peer = conn.remote_id();
    let protocol = String::from_utf8_lossy(conn.alpn());
    let paths = conn.paths();
    if paths.is_empty() {
        info!(connection, %peer, %protocol, event, "connection path unavailable");
    }
    for path in &paths {
        let stats = path.stats();
        info!(connection, %peer, %protocol, event,
            transport = transport_name(path.remote_addr()), remote = %path.remote_addr(),
            selected = path.is_selected(), closed = false,
            sent_bytes = stats.udp_tx.bytes, received_bytes = stats.udp_rx.bytes,
            "connection path");
    }
}

async fn accept_incoming(incoming: iroh::endpoint::Incoming) -> Option<AcceptedBi> {
    let mut connecting = match incoming.accept() {
        Ok(c) => c,
        Err(e) => {
            warn!("rejected incoming QUIC handshake: {e}");
            return None;
        }
    };
    let alpn = match connecting.alpn().await {
        Ok(a) => a,
        Err(e) => {
            warn!("could not read ALPN from incoming connection: {e}");
            return None;
        }
    };
    let conn = match connecting.await {
        Ok(c) => c,
        Err(e) => {
            warn!("incoming connection handshake failed: {e}");
            return None;
        }
    };
    let peer = conn.remote_id().to_node_id();
    let (send, recv) = match conn.accept_bi().await {
        Ok(s) => s,
        Err(e) => {
            warn!("accept_bi failed: {e}");
            return None;
        }
    };
    Some(AcceptedBi {
        peer,
        alpn,
        send: Box::new(send),
        recv: Box::new(recv),
        handle: ConnHandle { conn },
    })
}

async fn dispatch(accepted: AcceptedBi, store: Arc<StoreServer>) -> Result<()> {
    let AcceptedBi {
        peer,
        alpn,
        send,
        recv,
        handle,
    } = accepted;
    match alpn.as_slice() {
        ALPN_SYNC => {
            run_sync_session(false, peer, &store, recv, send).await?;
        }
        ALPN_JOIN => {
            run_join_host(peer, &store, recv, send).await?;
        }
        other => {
            return Err(UnbillError::Network(format!(
                "unknown ALPN from {peer}: {:?}",
                String::from_utf8_lossy(other)
            )));
        }
    }
    // The initiator closes only after reading the response.
    handle.wait_for_close().await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    async fn local_endpoint() -> iroh::Endpoint {
        iroh::Endpoint::builder(presets::Minimal)
            .clear_ip_transports()
            .bind_addr("127.0.0.1:0")
            .expect("loopback address")
            .alpns(vec![ALPN_SYNC.to_vec()])
            .bind()
            .await
            .expect("bind local endpoint")
    }

    #[tokio::test]
    async fn mdns_connects_by_node_id_without_relays_or_supplied_addresses() {
        let _ = tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .with_test_writer()
            .try_init();
        tokio::time::timeout(Duration::from_secs(20), async {
            let host = iroh::Endpoint::builder(presets::Minimal)
                .relay_mode(RelayMode::Disabled)
                .address_lookup(MdnsAddressLookup::builder())
                .alpns(vec![ALPN_SYNC.to_vec()])
                .bind()
                .await
                .expect("bind mDNS host");
            let client = iroh::Endpoint::builder(presets::Minimal)
                .relay_mode(RelayMode::Disabled)
                .address_lookup(MdnsAddressLookup::builder())
                .bind()
                .await
                .expect("bind mDNS client");
            let expected_peer = client.id();
            let accepting = host.clone();
            let server = tokio::spawn(async move {
                let conn = accepting
                    .accept()
                    .await
                    .expect("incoming connection")
                    .await
                    .expect("authenticate mDNS peer");
                assert_eq!(conn.remote_id(), expected_peer);
                let (mut send, mut recv) = conn.accept_bi().await.expect("accept stream");
                let mut request = [0; 4];
                recv.read_exact(&mut request).await.expect("read request");
                assert_eq!(&request, b"mdns");
                send.write_all(b"ok").await.expect("write reply");
                send.finish().expect("finish reply");
                conn.closed().await;
            });
            // Only the ID is supplied: mDNS is the sole source of a dialable address.
            let conn = match client.connect(host.id(), ALPN_SYNC).await {
                Ok(conn) => conn,
                Err(err) => {
                    server.abort();
                    client.close().await;
                    host.close().await;
                    panic!(
                        "resolve via mDNS (requires local-network permission and multicast): {err}"
                    );
                }
            };
            assert!(conn.paths().iter().any(|path| path.is_ip()));
            assert!(conn.paths().iter().all(|path| !path.is_relay()));
            let (mut send, mut recv) = conn.open_bi().await.expect("open stream");
            send.write_all(b"mdns").await.expect("write request");
            let mut reply = [0; 2];
            recv.read_exact(&mut reply).await.expect("read reply");
            assert_eq!(&reply, b"ok");
            conn.close(0u32.into(), b"done");
            server.await.expect("server task");
            client.close().await;
            host.close().await;
        })
        .await
        .expect("mDNS discovery must work without a relay");
    }

    #[tokio::test]
    async fn path_logging_does_not_keep_a_connection_alive() {
        tokio::time::timeout(Duration::from_secs(10), async {
            let host = local_endpoint().await;
            let accepting = host.clone();
            let server = tokio::spawn(async move {
                accepting
                    .accept()
                    .await
                    .expect("incoming")
                    .await
                    .expect("handshake")
            });
            let address = iroh::EndpointAddr::from_parts(
                host.id(),
                host.bound_sockets()
                    .into_iter()
                    .map(iroh::TransportAddr::Ip),
            );
            let client = local_endpoint().await;
            let conn = client.connect(address, ALPN_SYNC).await.expect("connect");
            let remote = server.await.expect("accept task");
            monitor_paths(&conn);
            drop(conn);
            remote.closed().await;
            host.close().await;
            client.close().await;
        })
        .await
        .expect("logging must not retain the last connection handle");
    }

    #[tokio::test]
    async fn idle_connection_does_not_block_later_connections() {
        tokio::time::timeout(Duration::from_secs(10), async {
            let host = Arc::new(UnbillEndpoint {
                inner: local_endpoint().await,
                addr_cache: MemoryLookup::new(),
                incoming_tasks: tokio::sync::Mutex::new(tokio::task::JoinSet::new()),
            });
            let address = iroh::EndpointAddr::from_parts(
                host.inner.id(),
                host.inner
                    .bound_sockets()
                    .into_iter()
                    .map(iroh::TransportAddr::Ip),
            );
            let accepting = Arc::clone(&host);
            let mut first_accept = Some(tokio::spawn(async move { accepting.accept_bi().await }));
            let idle_client = local_endpoint().await;
            let idle_connection = idle_client
                .connect(address.clone(), ALPN_SYNC)
                .await
                .expect("connect peer that never opens a stream");

            for attempt in 0..2 {
                let accept_task = if attempt == 0 {
                    first_accept.take().expect("first accept task")
                } else {
                    let accepting = Arc::clone(&host);
                    tokio::spawn(async move { accepting.accept_bi().await })
                };
                let client = local_endpoint().await;
                let conn = client
                    .connect(address.clone(), ALPN_SYNC)
                    .await
                    .expect("connect active peer");
                let (mut send, _recv) = conn.open_bi().await.expect("open stream");
                send.write_all(b"ready").await.expect("announce stream");
                let accepted = accept_task
                    .await
                    .expect("accept task")
                    .expect("accept active peer");
                assert_eq!(accepted.peer, client.id().to_node_id());
                accepted.handle.close();
                conn.close(0u32.into(), b"done");
                client.close().await;
            }
            idle_connection.close(0u32.into(), b"done");
            idle_client.close().await;
            host.close().await;
        })
        .await
        .expect("idle connection must not stall acceptance");
    }
}
