use async_trait::async_trait;
use tokio::sync::broadcast;
use unbill_model::error::Result;
use unbill_model::{LedgerId, LedgerMeta, NodeId};

#[cfg(feature = "http")]
pub mod http;
#[cfg(feature = "local")]
pub mod local;
#[cfg(feature = "rpc")]
pub mod rpc;

/// Events pushed from the device to all connected consoles.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type")]
pub enum AsymChannelEvent {
    LedgerUpdated { ledger_id: LedgerId },
}

// On wasm32 (single-threaded), relax Send + Sync requirements so that
// browser-native futures (reqwest / JsFuture, which contain !Send Rc) compile.
#[cfg(not(target_arch = "wasm32"))]
pub trait MaybeSend: Send {}
#[cfg(not(target_arch = "wasm32"))]
impl<T: Send> MaybeSend for T {}
#[cfg(target_arch = "wasm32")]
pub trait MaybeSend {}
#[cfg(target_arch = "wasm32")]
impl<T> MaybeSend for T {}

#[cfg(not(target_arch = "wasm32"))]
pub trait MaybeSync: Sync {}
#[cfg(not(target_arch = "wasm32"))]
impl<T: Sync> MaybeSync for T {}
#[cfg(target_arch = "wasm32")]
pub trait MaybeSync {}
#[cfg(target_arch = "wasm32")]
impl<T> MaybeSync for T {}

/// The asymmetric channel between a device and a console.
///
/// Control plane: invite / join / peer sync (request-response).
/// Data plane: one round of Automerge sync per call.
/// Metadata plane: ledger list, ledger meta, device meta.
/// Subscription: broadcast receiver the device pushes into when state changes.
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
pub trait AsymChannel: MaybeSend + MaybeSync {
    // --- Identity ---

    /// Return the device's node ID.
    fn device_id(&self) -> NodeId;

    // --- Control plane ---

    async fn create_invitation(&self, ledger_id: LedgerId) -> Result<String>;
    async fn join_ledger(&self, url: String, label: Option<String>) -> Result<()>;
    async fn trigger_peer_sync(&self, peer: NodeId) -> Result<()>;

    // --- Data plane ---

    /// One round of Automerge sync. The console sends its sync message bytes;
    /// the device responds with its own, or `None` if it has nothing new.
    async fn asym_sync(&self, ledger_id: LedgerId, bytes: Vec<u8>) -> Result<Option<Vec<u8>>>;

    // --- Metadata plane ---

    async fn list_ledgers(&self) -> Result<Vec<LedgerMeta>>;
    async fn save_ledger_meta(&self, meta: &LedgerMeta) -> Result<()>;
    async fn load_device_meta(&self, key: &str) -> Result<Option<Vec<u8>>>;
    async fn save_device_meta(&self, key: &str, bytes: Vec<u8>) -> Result<()>;

    // --- Event subscription ---

    /// Returns a broadcast receiver. The device sends into this channel
    /// whenever its ledger state changes (e.g. after a peer sync).
    fn subscribe_to_server(&self) -> broadcast::Receiver<AsymChannelEvent>;
}
