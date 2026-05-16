// tarpc RPC server and client for the AsymChannel trait.
//
// The service uses String for errors so all types are serializable.
// Event subscription uses a per-connection queue populated by a background
// task; the client polls it at a fixed interval and feeds its own broadcast.

use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use futures_util::StreamExt as _;
use interprocess::local_socket::tokio::prelude::*;
use interprocess::local_socket::{GenericFilePath, ListenerOptions};
use serde::{Deserialize, Serialize};
use tarpc::server::{BaseChannel, Channel as _};
use tarpc::tokio_serde::formats::Json;
use tokio::sync::broadcast;
use tokio_util::codec::LengthDelimitedCodec;
use unbill_model::error::{Result, UnbillError};
use unbill_model::{Currency, LedgerId, LedgerMeta, NodeId, Timestamp};

use crate::{AsymChannel, AsymChannelEvent};

// sirno:witness:asymmetric-channel:begin
// ---------------------------------------------------------------------------
// Wire types
// ---------------------------------------------------------------------------

/// Serializable mirror of `LedgerMeta` (Currency has no serde impl).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WireLedgerMeta {
    pub ledger_id: LedgerId,
    pub name: String,
    pub currency: String,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

impl WireLedgerMeta {
    fn from_meta(m: &LedgerMeta) -> Self {
        Self {
            ledger_id: m.ledger_id,
            name: m.name.clone(),
            currency: m.currency.code().to_owned(),
            created_at_ms: m.created_at.as_millis(),
            updated_at_ms: m.updated_at.as_millis(),
        }
    }

    fn into_meta(self) -> Result<LedgerMeta> {
        let currency = Currency::from_code(&self.currency)
            .ok_or_else(|| UnbillError::Network(format!("unknown currency {:?}", self.currency)))?;
        Ok(LedgerMeta {
            ledger_id: self.ledger_id,
            name: self.name,
            currency,
            created_at: Timestamp::from_millis(self.created_at_ms),
            updated_at: Timestamp::from_millis(self.updated_at_ms),
        })
    }
}

// ---------------------------------------------------------------------------
// tarpc service definition
// ---------------------------------------------------------------------------

#[tarpc::service]
pub trait AsymChannelService {
    async fn get_device_id() -> String;
    async fn create_invitation(ledger_id: LedgerId) -> std::result::Result<String, String>;
    async fn join_ledger(url: String, label: Option<String>) -> std::result::Result<(), String>;
    async fn trigger_peer_sync(peer: NodeId) -> std::result::Result<(), String>;
    async fn asym_sync(
        ledger_id: LedgerId,
        bytes: Vec<u8>,
    ) -> std::result::Result<Option<Vec<u8>>, String>;
    async fn list_ledgers() -> std::result::Result<Vec<WireLedgerMeta>, String>;
    async fn save_ledger_meta(meta: WireLedgerMeta) -> std::result::Result<(), String>;
    async fn load_device_meta(key: String) -> std::result::Result<Option<Vec<u8>>, String>;
    async fn save_device_meta(key: String, bytes: Vec<u8>) -> std::result::Result<(), String>;
    /// Poll and drain pending device events for this connection.
    async fn poll_events() -> Vec<WireEvent>;
}
// sirno:witness:asymmetric-channel:end

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

// sirno:witness:asymmetric-channel:begin
struct AsymChannelServiceServer<C: AsymChannel> {
    channel: Arc<C>,
    /// Per-connection event queue populated by a background subscriber task.
    event_queue: Arc<Mutex<Vec<WireEvent>>>,
}

impl<C: AsymChannel> Clone for AsymChannelServiceServer<C> {
    fn clone(&self) -> Self {
        Self {
            channel: Arc::clone(&self.channel),
            event_queue: Arc::clone(&self.event_queue),
        }
    }
}

impl<C: AsymChannel> AsymChannelService for AsymChannelServiceServer<C> {
    async fn get_device_id(self, _ctx: tarpc::context::Context) -> String {
        self.channel.device_id().to_string()
    }

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
        label: Option<String>,
    ) -> std::result::Result<(), String> {
        self.channel
            .join_ledger(url, label)
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

    async fn list_ledgers(
        self,
        _ctx: tarpc::context::Context,
    ) -> std::result::Result<Vec<WireLedgerMeta>, String> {
        self.channel
            .list_ledgers()
            .await
            .map(|v| v.iter().map(WireLedgerMeta::from_meta).collect())
            .map_err(|e| e.to_string())
    }

    async fn save_ledger_meta(
        self,
        _ctx: tarpc::context::Context,
        meta: WireLedgerMeta,
    ) -> std::result::Result<(), String> {
        let meta = meta.into_meta().map_err(|e| e.to_string())?;
        self.channel
            .save_ledger_meta(&meta)
            .await
            .map_err(|e| e.to_string())
    }

    async fn load_device_meta(
        self,
        _ctx: tarpc::context::Context,
        key: String,
    ) -> std::result::Result<Option<Vec<u8>>, String> {
        self.channel
            .load_device_meta(&key)
            .await
            .map_err(|e| e.to_string())
    }

    async fn save_device_meta(
        self,
        _ctx: tarpc::context::Context,
        key: String,
        bytes: Vec<u8>,
    ) -> std::result::Result<(), String> {
        self.channel
            .save_device_meta(&key, bytes)
            .await
            .map_err(|e| e.to_string())
    }

    async fn poll_events(self, _ctx: tarpc::context::Context) -> Vec<WireEvent> {
        std::mem::take(&mut *self.event_queue.lock().unwrap())
    }
}

/// Binds a local socket at `path`, removing any stale file first.
/// Deletes the socket file on drop.
struct LocalSocketGuard {
    listener: LocalSocketListener,
    path: std::path::PathBuf,
}

impl LocalSocketGuard {
    fn bind(path: &Path) -> std::io::Result<Self> {
        // Remove stale socket from a previous run.
        if let Err(e) = std::fs::remove_file(path)
            && e.kind() != std::io::ErrorKind::NotFound
        {
            return Err(e);
        }
        let name = path.to_fs_name::<GenericFilePath>()?;
        let listener = ListenerOptions::new().name(name).create_tokio()?;
        Ok(Self {
            listener,
            path: path.to_owned(),
        })
    }
}

impl Drop for LocalSocketGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

/// Serve an `AsymChannel` over tarpc on the given local socket path.
/// Each accepted connection gets its own event queue and subscriber task.
pub async fn serve<C>(channel: Arc<C>, path: &Path) -> std::io::Result<()>
where
    C: AsymChannel + 'static,
{
    let guard = LocalSocketGuard::bind(path)?;

    loop {
        let stream = guard.listener.accept().await?;
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

        let transport = tarpc::serde_transport::new(
            LengthDelimitedCodec::builder().new_framed(stream),
            Json::default(),
        );
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
    }
}
// sirno:witness:asymmetric-channel:end

// ---------------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------------

/// Wraps the generated tarpc client and implements the AsymChannel trait.
/// Polls `poll_events` in a background task to feed the broadcast channel.
pub struct RpcAsymChannel {
    client: AsymChannelServiceClient,
    device_node_id: NodeId,
    events: broadcast::Sender<AsymChannelEvent>,
}

impl RpcAsymChannel {
    pub async fn connect(path: &Path) -> std::io::Result<Arc<Self>> {
        let name = path.to_fs_name::<GenericFilePath>()?;
        let stream = LocalSocketStream::connect(name).await?;
        let transport = tarpc::serde_transport::new(
            LengthDelimitedCodec::builder().new_framed(stream),
            Json::default(),
        );
        let client =
            AsymChannelServiceClient::new(tarpc::client::Config::default(), transport).spawn();

        let id_str = client
            .get_device_id(tarpc::context::current())
            .await
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        let device_node_id = NodeId::new(id_str);

        let (tx, _) = broadcast::channel(256);
        let rpc = Arc::new(Self {
            client: client.clone(),
            device_node_id,
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
    fn device_id(&self) -> NodeId {
        self.device_node_id.clone()
    }

    async fn create_invitation(&self, ledger_id: LedgerId) -> Result<String> {
        self.client
            .create_invitation(tarpc::context::current(), ledger_id)
            .await
            .map_err(|e| UnbillError::Network(e.to_string()))?
            .map_err(UnbillError::Network)
    }

    async fn join_ledger(&self, url: String, label: Option<String>) -> Result<()> {
        self.client
            .join_ledger(tarpc::context::current(), url, label)
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

    async fn list_ledgers(&self) -> Result<Vec<LedgerMeta>> {
        self.client
            .list_ledgers(tarpc::context::current())
            .await
            .map_err(|e| UnbillError::Network(e.to_string()))?
            .map_err(UnbillError::Network)?
            .into_iter()
            .map(WireLedgerMeta::into_meta)
            .collect()
    }

    async fn save_ledger_meta(&self, meta: &LedgerMeta) -> Result<()> {
        self.client
            .save_ledger_meta(tarpc::context::current(), WireLedgerMeta::from_meta(meta))
            .await
            .map_err(|e| UnbillError::Network(e.to_string()))?
            .map_err(UnbillError::Network)
    }

    async fn load_device_meta(&self, key: &str) -> Result<Option<Vec<u8>>> {
        self.client
            .load_device_meta(tarpc::context::current(), key.to_owned())
            .await
            .map_err(|e| UnbillError::Network(e.to_string()))?
            .map_err(UnbillError::Network)
    }

    async fn save_device_meta(&self, key: &str, bytes: Vec<u8>) -> Result<()> {
        self.client
            .save_device_meta(tarpc::context::current(), key.to_owned(), bytes)
            .await
            .map_err(|e| UnbillError::Network(e.to_string()))?
            .map_err(UnbillError::Network)
    }

    fn subscribe_to_server(&self) -> broadcast::Receiver<AsymChannelEvent> {
        self.events.subscribe()
    }
}
