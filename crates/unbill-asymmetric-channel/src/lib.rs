use async_trait::async_trait;
use tokio::sync::broadcast;
use unbill_model::error::Result;
use unbill_model::{LedgerId, NodeId};

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

/// The asymmetric channel between a device and a console.
///
/// Control plane: invite / join / peer sync (request-response).
/// Data plane: one round of Automerge sync per call.
/// Subscription: broadcast receiver the device pushes into when state changes.
#[async_trait]
pub trait AsymChannel: Send + Sync {
    // --- Control plane ---

    async fn create_invitation(&self, ledger_id: LedgerId) -> Result<String>;
    async fn join_ledger(&self, url: String) -> Result<()>;
    async fn trigger_peer_sync(&self, peer: NodeId) -> Result<()>;

    // --- Data plane ---

    /// One round of Automerge sync. The console sends its sync message bytes;
    /// the device responds with its own, or `None` if it has nothing new.
    async fn asym_sync(&self, ledger_id: LedgerId, bytes: Vec<u8>) -> Result<Option<Vec<u8>>>;

    // --- Event subscription ---

    /// Returns a broadcast receiver. The device sends into this channel
    /// whenever its ledger state changes (e.g. after a peer sync).
    fn subscribe_to_server(&self) -> broadcast::Receiver<AsymChannelEvent>;
}
