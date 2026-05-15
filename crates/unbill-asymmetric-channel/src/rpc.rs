// tarpc RPC server and client for the AsymChannel trait.
//
// The service uses String for errors so all types are serializable.
// Event subscription uses a per-connection queue populated by a background
// task; the client polls it at a fixed interval and feeds its own broadcast.

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use futures_util::{StreamExt as _, future};
use serde::{Deserialize, Serialize};
use tarpc::server::{BaseChannel, Channel as _};
use tarpc::tokio_serde::formats::Json;
use tokio::sync::broadcast;
use unbill_model::error::{Result, UnbillError};
use unbill_model::{LedgerId, NodeId};

use crate::{AsymChannel, AsymChannelEvent};

// ---------------------------------------------------------------------------
// tarpc service definition
// ---------------------------------------------------------------------------

#[tarpc::service]
pub trait AsymChannelService {
    async fn create_invitation(ledger_id: LedgerId) -> std::result::Result<String, String>;
    async fn join_ledger(url: String) -> std::result::Result<(), String>;
    async fn trigger_peer_sync(peer: NodeId) -> std::result::Result<(), String>;
    async fn asym_sync(
        ledger_id: LedgerId,
        bytes: Vec<u8>,
    ) -> std::result::Result<Option<Vec<u8>>, String>;
    /// Poll and drain pending device events for this connection.
    async fn poll_events() -> Vec<WireEvent>;
}

// ---------------------------------------------------------------------------
// Wire event (serializable mirror of AsymChannelEvent)
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WireEvent {
    LedgerUpdated { ledger_id: LedgerId },
}

impl From<AsymChannelEvent> for WireEvent {
    fn from(e: AsymChannelEvent) -> Self {
        match e {
            AsymChannelEvent::LedgerUpdated { ledger_id } => WireEvent::LedgerUpdated { ledger_id },
        }
    }
}

impl From<WireEvent> for AsymChannelEvent {
    fn from(e: WireEvent) -> Self {
        match e {
            WireEvent::LedgerUpdated { ledger_id } => AsymChannelEvent::LedgerUpdated { ledger_id },
        }
    }
}

// ---------------------------------------------------------------------------
// Server
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct AsymChannelServiceServer<C: AsymChannel> {
    channel: Arc<C>,
    /// Per-connection event queue populated by a background subscriber task.
    event_queue: Arc<Mutex<Vec<WireEvent>>>,
}

impl<C: AsymChannel + Clone> AsymChannelService for AsymChannelServiceServer<C> {
    async fn create_invitation(
        self,
        _ctx: tarpc::context::Context,
        ledger_id: LedgerId,
    ) -> std::result::Result<String, String> {
        self.channel
            .create_invitation(ledger_id)
            .await
            .map_err(|e| e.to_string())
    }

    async fn join_ledger(
        self,
        _ctx: tarpc::context::Context,
        url: String,
    ) -> std::result::Result<(), String> {
        self.channel
            .join_ledger(url)
            .await
            .map_err(|e| e.to_string())
    }

    async fn trigger_peer_sync(
        self,
        _ctx: tarpc::context::Context,
        peer: NodeId,
    ) -> std::result::Result<(), String> {
        self.channel
            .trigger_peer_sync(peer)
            .await
            .map_err(|e| e.to_string())
    }

    async fn asym_sync(
        self,
        _ctx: tarpc::context::Context,
        ledger_id: LedgerId,
        bytes: Vec<u8>,
    ) -> std::result::Result<Option<Vec<u8>>, String> {
        self.channel
            .asym_sync(ledger_id, bytes)
            .await
            .map_err(|e| e.to_string())
    }

    async fn poll_events(self, _ctx: tarpc::context::Context) -> Vec<WireEvent> {
        std::mem::take(&mut *self.event_queue.lock().unwrap())
    }
}

/// Serve an `AsymChannel` over tarpc on the given address.
/// Each accepted connection gets its own event queue and subscriber task.
pub async fn serve<C>(channel: Arc<C>, addr: SocketAddr) -> std::io::Result<()>
where
    C: AsymChannel + Clone + 'static,
{
    let listener = tarpc::serde_transport::tcp::listen(&addr, Json::default).await?;
    listener
        .filter_map(|r| future::ready(r.ok()))
        .for_each(|transport| {
            let channel = Arc::clone(&channel);
            let queue: Arc<Mutex<Vec<WireEvent>>> = Arc::new(Mutex::new(Vec::new()));

            // Background task: subscribe and push events into this connection's queue.
            let mut rx = channel.subscribe_to_server();
            let q2 = Arc::clone(&queue);
            tokio::spawn(async move {
                loop {
                    match rx.recv().await {
                        Ok(evt) => q2.lock().unwrap().push(WireEvent::from(evt)),
                        Err(broadcast::error::RecvError::Lagged(_)) => continue,
                        Err(broadcast::error::RecvError::Closed) => break,
                    }
                }
            });

            let server = AsymChannelServiceServer {
                channel,
                event_queue: queue,
            };
            tokio::spawn(
                BaseChannel::with_defaults(transport)
                    .execute(server.serve())
                    .for_each(|f| async move {
                        tokio::spawn(f);
                    }),
            );
            future::ready(())
        })
        .await;
    Ok(())
}

// ---------------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------------

/// Wraps the generated tarpc client and implements the AsymChannel trait.
/// Polls `poll_events` in a background task to feed the broadcast channel.
pub struct RpcAsymChannel {
    client: AsymChannelServiceClient,
    events: broadcast::Sender<AsymChannelEvent>,
}

impl RpcAsymChannel {
    pub async fn connect(addr: SocketAddr) -> std::io::Result<Arc<Self>> {
        let transport = tarpc::serde_transport::tcp::connect(addr, Json::default).await?;
        let client =
            AsymChannelServiceClient::new(tarpc::client::Config::default(), transport).spawn();

        let (tx, _) = broadcast::channel(256);
        let rpc = Arc::new(Self {
            client: client.clone(),
            events: tx,
        });

        // Background polling task.
        let tx2 = rpc.events.clone();
        tokio::spawn(async move {
            loop {
                if let Ok(events) = client.poll_events(tarpc::context::current()).await {
                    for wire in events {
                        let _ = tx2.send(AsymChannelEvent::from(wire));
                    }
                }
                tokio::time::sleep(Duration::from_millis(200)).await;
            }
        });

        Ok(rpc)
    }
}

#[async_trait]
impl AsymChannel for RpcAsymChannel {
    async fn create_invitation(&self, ledger_id: LedgerId) -> Result<String> {
        self.client
            .create_invitation(tarpc::context::current(), ledger_id)
            .await
            .map_err(|e| UnbillError::Network(e.to_string()))?
            .map_err(UnbillError::Network)
    }

    async fn join_ledger(&self, url: String) -> Result<()> {
        self.client
            .join_ledger(tarpc::context::current(), url)
            .await
            .map_err(|e| UnbillError::Network(e.to_string()))?
            .map_err(UnbillError::Network)
    }

    async fn trigger_peer_sync(&self, peer: NodeId) -> Result<()> {
        self.client
            .trigger_peer_sync(tarpc::context::current(), peer)
            .await
            .map_err(|e| UnbillError::Network(e.to_string()))?
            .map_err(UnbillError::Network)
    }

    async fn asym_sync(&self, ledger_id: LedgerId, bytes: Vec<u8>) -> Result<Option<Vec<u8>>> {
        self.client
            .asym_sync(tarpc::context::current(), ledger_id, bytes)
            .await
            .map_err(|e| UnbillError::Network(e.to_string()))?
            .map_err(UnbillError::Network)
    }

    fn subscribe_to_server(&self) -> broadcast::Receiver<AsymChannelEvent> {
        self.events.subscribe()
    }
}
