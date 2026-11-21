// tarpc RPC server and client for the AsymChannel trait.
//
// The service uses String for errors so all types are serializable.
// Event subscription uses a per-connection queue populated by a background
// task; the client polls it at a fixed interval and feeds its own broadcast.

use std::path::Path;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

#[cfg(feature = "local")]
use crate::local::LocalAsymChannel;
use async_trait::async_trait;
use futures_util::StreamExt as _;
use interprocess::local_socket::tokio::prelude::*;
use interprocess::local_socket::{GenericFilePath, ListenerOptions};
use serde::{Deserialize, Serialize};
use tarpc::server::{BaseChannel, Channel as _};
use tarpc::tokio_serde::formats::Json;
use tokio::sync::broadcast;
use tokio_util::codec::LengthDelimitedCodec;
#[cfg(feature = "local")]
use unbill_device::{AccountView, UnbillDevice};
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
    #[serde(default)]
    pub archived: bool,
    pub ledger_id: LedgerId,
    pub name: String,
    pub currency: String,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

impl WireLedgerMeta {
    fn from_meta(m: &LedgerMeta) -> Self {
        Self {
            archived: m.archived,
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
            archived: self.archived,
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
    async fn register(name: String, password: String) -> std::result::Result<(), String>;
    async fn login(name: String, password: String) -> std::result::Result<String, String>;
    async fn resume_session(token: String) -> std::result::Result<String, String>;
    async fn current_account() -> std::result::Result<String, String>;
    async fn logout() -> std::result::Result<(), String>;
    async fn change_password(old: String, new: String) -> std::result::Result<(), String>;
    async fn list_accounts() -> std::result::Result<Vec<String>, String>;
    async fn share_ledger(
        ledger_id: LedgerId,
        account_name: String,
    ) -> std::result::Result<(), String>;
    async fn get_device_id() -> String;
    async fn build_info() -> std::result::Result<unbill_build_info::BuildInfo, String>;
    async fn ble_snapshot() -> std::result::Result<unbill_model::BleSnapshot, String>;
    async fn create_invitation(ledger_id: LedgerId) -> std::result::Result<String, String>;
    async fn join_ledger(
        url: String,
        label: unbill_model::DeviceLabel,
    ) -> std::result::Result<(), String>;
    async fn trigger_peer_sync(peer: NodeId) -> std::result::Result<(), String>;
    async fn asym_sync(
        ledger_id: LedgerId,
        bytes: Vec<u8>,
    ) -> std::result::Result<Option<Vec<u8>>, String>;
    async fn list_ledgers() -> std::result::Result<Vec<WireLedgerMeta>, String>;
    async fn set_ledger_archived(
        ledger_id: LedgerId,
        archived: bool,
    ) -> std::result::Result<(), String>;
    async fn save_ledger_meta(meta: WireLedgerMeta) -> std::result::Result<(), String>;

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
    BleSnapshotChanged,
    LedgerUpdated { ledger_id: LedgerId },
}

impl From<AsymChannelEvent> for WireEvent {
    fn from(e: AsymChannelEvent) -> Self {
        match e {
            AsymChannelEvent::BleSnapshotChanged => WireEvent::BleSnapshotChanged,
            AsymChannelEvent::LedgerUpdated { ledger_id } => WireEvent::LedgerUpdated { ledger_id },
        }
    }
}

impl From<WireEvent> for AsymChannelEvent {
    fn from(e: WireEvent) -> Self {
        match e {
            WireEvent::BleSnapshotChanged => AsymChannelEvent::BleSnapshotChanged,
            WireEvent::LedgerUpdated { ledger_id } => AsymChannelEvent::LedgerUpdated { ledger_id },
        }
    }
}

// ---------------------------------------------------------------------------
// Server
// ---------------------------------------------------------------------------

// sirno:witness:asymmetric-channel:begin
#[cfg(feature = "local")]
struct AsymChannelServiceServer {
    channel: Arc<LocalAsymChannel>,
    session: Arc<Mutex<Option<String>>>,
    /// Per-connection event queue populated by a background subscriber task.
    /// Only push and take run under the lock, so the Vec can be reused after poisoning.
    event_queue: Arc<Mutex<Vec<WireEvent>>>,
}

#[cfg(feature = "local")]
impl Clone for AsymChannelServiceServer {
    fn clone(&self) -> Self {
        Self {
            channel: Arc::clone(&self.channel),
            session: Arc::clone(&self.session),
            event_queue: Arc::clone(&self.event_queue),
        }
    }
}

#[cfg(feature = "local")]
impl AsymChannelServiceServer {
    fn device(&self) -> &Arc<UnbillDevice> {
        self.channel.service()
    }
    async fn view(&self) -> std::result::Result<AccountView, String> {
        let token = self
            .session
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
            .ok_or_else(|| "not authorized".to_owned())?;
        self.device()
            .session_view(&token)
            .await
            .map_err(|e| e.to_string())
    }
}

#[cfg(feature = "local")]
impl AsymChannelService for AsymChannelServiceServer {
    async fn register(
        self,
        _ctx: tarpc::context::Context,
        name: String,
        password: String,
    ) -> std::result::Result<(), String> {
        self.device()
            .register(&name, &password)
            .await
            .map_err(|e| e.to_string())
    }
    async fn login(
        self,
        _ctx: tarpc::context::Context,
        name: String,
        password: String,
    ) -> std::result::Result<String, String> {
        let token = self
            .device()
            .login(&name, &password)
            .await
            .map_err(|e| e.to_string())?;
        let previous = self
            .session
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .replace(token.clone());
        if let Some(previous) = previous {
            self.device()
                .logout(&previous)
                .await
                .map_err(|e| e.to_string())?;
        }
        Ok(token)
    }
    async fn resume_session(
        self,
        _ctx: tarpc::context::Context,
        token: String,
    ) -> std::result::Result<String, String> {
        let name = self
            .device()
            .session_account(&token)
            .await
            .map_err(|e| e.to_string())?;
        *self.session.lock().unwrap_or_else(PoisonError::into_inner) = Some(token);
        Ok(name)
    }
    async fn current_account(
        self,
        _ctx: tarpc::context::Context,
    ) -> std::result::Result<String, String> {
        Ok(self.view().await?.account_name().to_owned())
    }
    async fn logout(self, _ctx: tarpc::context::Context) -> std::result::Result<(), String> {
        let token = self
            .session
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take()
            .ok_or_else(|| "not authorized".to_owned())?;
        self.device()
            .logout(&token)
            .await
            .map_err(|e| e.to_string())
    }
    async fn change_password(
        self,
        _ctx: tarpc::context::Context,
        old: String,
        new: String,
    ) -> std::result::Result<(), String> {
        let token = self
            .session
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
            .ok_or_else(|| "not authorized".to_owned())?;
        self.device()
            .change_password(&token, &old, &new)
            .await
            .map_err(|e| e.to_string())?;
        *self.session.lock().unwrap_or_else(PoisonError::into_inner) = None;
        Ok(())
    }
    async fn list_accounts(
        self,
        _ctx: tarpc::context::Context,
    ) -> std::result::Result<Vec<String>, String> {
        self.view()
            .await?
            .list_accounts()
            .await
            .map_err(|e| e.to_string())
    }
    async fn share_ledger(
        self,
        _ctx: tarpc::context::Context,
        id: LedgerId,
        name: String,
    ) -> std::result::Result<(), String> {
        self.view()
            .await?
            .share_ledger(id, &name)
            .await
            .map_err(|e| e.to_string())
    }
    async fn ble_snapshot(
        self,
        _ctx: tarpc::context::Context,
    ) -> std::result::Result<unbill_model::BleSnapshot, String> {
        self.view().await?;
        self.channel.ble_snapshot().await.map_err(|e| e.to_string())
    }

    async fn get_device_id(self, _ctx: tarpc::context::Context) -> String {
        self.channel.device_id().to_string()
    }

    async fn build_info(
        self,
        _ctx: tarpc::context::Context,
    ) -> std::result::Result<unbill_build_info::BuildInfo, String> {
        self.channel.build_info().await.map_err(|e| e.to_string())
    }

    async fn create_invitation(
        self,
        _ctx: tarpc::context::Context,
        ledger_id: LedgerId,
    ) -> std::result::Result<String, String> {
        self.view()
            .await?
            .create_invitation(ledger_id)
            .await
            .map_err(|e| e.to_string())
    }

    async fn join_ledger(
        self,
        _ctx: tarpc::context::Context,
        url: String,
        label: unbill_model::DeviceLabel,
    ) -> std::result::Result<(), String> {
        self.view()
            .await?
            .join_ledger(&url, label)
            .await
            .map_err(|e| e.to_string())
    }

    async fn trigger_peer_sync(
        self,
        _ctx: tarpc::context::Context,
        peer: NodeId,
    ) -> std::result::Result<(), String> {
        self.view().await?;
        self.device()
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
        self.view()
            .await?
            .asym_sync(ledger_id, bytes)
            .await
            .map_err(|e| e.to_string())
    }

    async fn list_ledgers(
        self,
        _ctx: tarpc::context::Context,
    ) -> std::result::Result<Vec<WireLedgerMeta>, String> {
        self.view()
            .await?
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
        self.view()
            .await?
            .save_ledger_meta(&meta)
            .await
            .map_err(|e| e.to_string())
    }

    async fn set_ledger_archived(
        self,
        _ctx: tarpc::context::Context,
        ledger_id: LedgerId,
        archived: bool,
    ) -> std::result::Result<(), String> {
        self.view()
            .await?
            .set_ledger_archived(ledger_id, archived)
            .await
            .map_err(|e| e.to_string())
    }

    async fn poll_events(self, _ctx: tarpc::context::Context) -> Vec<WireEvent> {
        let Ok(view) = self.view().await else {
            return Vec::new();
        };
        let ids: Vec<_> = match view.list_ledgers().await {
            Ok(metas) => metas.into_iter().map(|meta| meta.ledger_id).collect(),
            Err(_) => return Vec::new(),
        };
        let mut queue = self
            .event_queue
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        std::mem::take(&mut *queue)
            .into_iter()
            .filter(|event| match event {
                WireEvent::LedgerUpdated { ledger_id } => ids.contains(ledger_id),
                WireEvent::BleSnapshotChanged => true,
            })
            .collect()
    }
}

/// Binds a local socket at `path`, removing any stale file first.
/// Deletes the socket file on drop.
#[cfg(feature = "local")]
struct LocalSocketGuard {
    listener: LocalSocketListener,
    path: std::path::PathBuf,
}

#[cfg(feature = "local")]
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

#[cfg(feature = "local")]
impl Drop for LocalSocketGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

/// Serve an `AsymChannel` over tarpc on the given local socket path.
/// Each accepted connection gets its own event queue and subscriber task.
#[cfg(feature = "local")]
pub async fn serve(channel: Arc<LocalAsymChannel>, path: &Path) -> std::io::Result<()> {
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
                    Ok(evt) => {
                        let mut queue = q2.lock().unwrap_or_else(PoisonError::into_inner);
                        // BLE notifications invalidate state; one pending item is enough.
                        if !matches!(evt, AsymChannelEvent::BleSnapshotChanged)
                            || !queue
                                .iter()
                                .any(|e| matches!(e, WireEvent::BleSnapshotChanged))
                        {
                            queue.push(WireEvent::from(evt));
                        }
                    }
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
            session: Arc::new(Mutex::new(None)),
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
    session_path: std::path::PathBuf,
    device_node_id: NodeId,
    events: broadcast::Sender<AsymChannelEvent>,
}

impl RpcAsymChannel {
    fn save_session(&self, token: &str) -> std::io::Result<()> {
        use std::io::Write as _;
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create(true).truncate(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt as _;
            options.mode(0o600);
        }
        let mut file = options.open(&self.session_path)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
        }
        file.write_all(token.as_bytes())
    }

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
        let session_path = path.with_extension("session");
        if let Ok(token) = std::fs::read_to_string(&session_path)
            && client
                .resume_session(tarpc::context::current(), token.trim().to_owned())
                .await
                .ok()
                .and_then(std::result::Result::ok)
                .is_none()
        {
            let _ = std::fs::remove_file(&session_path);
        }

        let (tx, _) = broadcast::channel(256);
        let rpc = Arc::new(Self {
            client: client.clone(),
            session_path,
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
    async fn register(&self, name: &str, password: &str) -> Result<()> {
        self.client
            .register(
                tarpc::context::current(),
                name.to_owned(),
                password.to_owned(),
            )
            .await
            .map_err(|e| UnbillError::Network(e.to_string()))?
            .map_err(UnbillError::Network)
    }
    async fn login(&self, name: &str, password: &str) -> Result<String> {
        let token = self
            .client
            .login(
                tarpc::context::current(),
                name.to_owned(),
                password.to_owned(),
            )
            .await
            .map_err(|e| UnbillError::Network(e.to_string()))?
            .map_err(UnbillError::Network)?;
        self.save_session(&token)
            .map_err(|e| UnbillError::Network(e.to_string()))?;
        Ok(token)
    }
    async fn resume_session(&self, token: &str) -> Result<String> {
        let name = self
            .client
            .resume_session(tarpc::context::current(), token.to_owned())
            .await
            .map_err(|e| UnbillError::Network(e.to_string()))?
            .map_err(UnbillError::Network)?;
        self.save_session(token)
            .map_err(|e| UnbillError::Network(e.to_string()))?;
        Ok(name)
    }
    async fn current_account(&self) -> Result<String> {
        self.client
            .current_account(tarpc::context::current())
            .await
            .map_err(|e| UnbillError::Network(e.to_string()))?
            .map_err(UnbillError::Network)
    }
    async fn logout(&self) -> Result<()> {
        self.client
            .logout(tarpc::context::current())
            .await
            .map_err(|e| UnbillError::Network(e.to_string()))?
            .map_err(UnbillError::Network)?;
        let _ = std::fs::remove_file(&self.session_path);
        Ok(())
    }
    async fn change_password(&self, old: &str, new: &str) -> Result<()> {
        self.client
            .change_password(tarpc::context::current(), old.to_owned(), new.to_owned())
            .await
            .map_err(|e| UnbillError::Network(e.to_string()))?
            .map_err(UnbillError::Network)?;
        let _ = std::fs::remove_file(&self.session_path);
        Ok(())
    }
    async fn list_accounts(&self) -> Result<Vec<String>> {
        self.client
            .list_accounts(tarpc::context::current())
            .await
            .map_err(|e| UnbillError::Network(e.to_string()))?
            .map_err(UnbillError::Network)
    }
    async fn share_ledger(&self, id: LedgerId, name: &str) -> Result<()> {
        self.client
            .share_ledger(tarpc::context::current(), id, name.to_owned())
            .await
            .map_err(|e| UnbillError::Network(e.to_string()))?
            .map_err(UnbillError::Network)
    }
    async fn ble_snapshot(&self) -> Result<unbill_model::BleSnapshot> {
        self.client
            .ble_snapshot(tarpc::context::current())
            .await
            .map_err(|e| UnbillError::Network(e.to_string()))?
            .map_err(UnbillError::Network)
    }

    fn device_id(&self) -> NodeId {
        self.device_node_id.clone()
    }

    async fn build_info(&self) -> Result<unbill_build_info::BuildInfo> {
        self.client
            .build_info(tarpc::context::current())
            .await
            .map_err(|e| UnbillError::Network(e.to_string()))?
            .map_err(UnbillError::Network)
    }

    async fn create_invitation(&self, ledger_id: LedgerId) -> Result<String> {
        self.client
            .create_invitation(tarpc::context::current(), ledger_id)
            .await
            .map_err(|e| UnbillError::Network(e.to_string()))?
            .map_err(UnbillError::Network)
    }

    async fn join_ledger(&self, url: String, label: unbill_model::DeviceLabel) -> Result<()> {
        self.client
            .join_ledger(tarpc::context::current(), url, label)
            .await
            .map_err(|e| UnbillError::Network(e.to_string()))?
            .map_err(UnbillError::Network)
    }

    async fn trigger_peer_sync(&self, peer: NodeId) -> Result<()> {
        // A peer connection and full exchange need more time than a local query.
        let mut context = tarpc::context::current();
        context.deadline = Instant::now()
            .checked_add(Duration::from_secs(120))
            .ok_or_else(|| UnbillError::Network("Could not set peer sync deadline".into()))?;
        self.client
            .trigger_peer_sync(context, peer)
            .await
            .map_err(|e| match e {
                tarpc::client::RpcError::DeadlineExceeded => {
                    UnbillError::Network("Peer sync timed out after 120 seconds".into())
                }
                other => UnbillError::Network(other.to_string()),
            })?
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

    async fn set_ledger_archived(&self, ledger_id: LedgerId, archived: bool) -> Result<()> {
        self.client
            .set_ledger_archived(tarpc::context::current(), ledger_id, archived)
            .await
            .map_err(|e| UnbillError::Network(e.to_string()))?
            .map_err(UnbillError::Network)
    }

    async fn save_ledger_meta(&self, meta: &LedgerMeta) -> Result<()> {
        self.client
            .save_ledger_meta(tarpc::context::current(), WireLedgerMeta::from_meta(meta))
            .await
            .map_err(|e| UnbillError::Network(e.to_string()))?
            .map_err(UnbillError::Network)
    }

    fn subscribe_to_server(&self) -> broadcast::Receiver<AsymChannelEvent> {
        self.events.subscribe()
    }
}
