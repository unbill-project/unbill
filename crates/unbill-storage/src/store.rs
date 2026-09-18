use async_trait::async_trait;
use std::collections::HashMap;
use tokio::sync::broadcast;
use unbill_event::ServiceEvent;
use unbill_model::Invitation;

use unbill_model::{LedgerId, LedgerMeta, NodeId, SecretKey, StorageError, Timestamp};

use unbill_model::LedgerDoc;

pub type StorageResult<T> = std::result::Result<T, StorageError>;

// sirno:witness:unbill-storage:begin
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
pub trait LedgerStore: Send + Sync {
    /// Create or update the per-ledger metadata cache.
    async fn save_ledger_meta(&self, meta: &LedgerMeta) -> StorageResult<()>;
    async fn list_ledgers(&self) -> StorageResult<Vec<LedgerMeta>>;

    /// Load a ledger document. Returns `None` if the ledger has never been saved.
    async fn load_ledger(&self, ledger_id: &str) -> StorageResult<Option<LedgerDoc>>;

    /// Persist a ledger document. A remote-aware store may apply changes back
    /// into `doc` before returning; callers must treat `doc` as the
    /// authoritative merged state after a successful call.
    async fn save_ledger(&self, ledger_id: &str, doc: &mut LedgerDoc) -> StorageResult<()>;

    async fn list_device_labels(&self) -> StorageResult<HashMap<String, String>>;
    /// Set one label, or remove it when `label` is None.
    async fn set_device_label(&self, node_id: &NodeId, label: Option<&str>) -> StorageResult<()>;
    async fn list_pending_invitations(&self) -> StorageResult<Vec<Invitation>>;
    /// Persist a new invitation with a freshly generated token; existing tokens cannot be resubmitted.
    async fn create_invitation(
        &self,
        ledger_id: LedgerId,
        created_by_device: &NodeId,
        created_at: Timestamp,
        expires_at: Timestamp,
    ) -> StorageResult<Invitation>;
    /// Atomically remove and return an invitation, preventing reuse.
    async fn consume_invitation(&self, token: &str) -> StorageResult<Option<Invitation>>;

    /// Generate a new random secret key and persist it.
    /// Idempotent: no-op if a key already exists.
    async fn create_secret_key(&self) -> StorageResult<()>;

    /// Returns `true` if a secret key (and thus a device identity) exists.
    async fn is_device_initialized(&self) -> StorageResult<bool>;

    /// Return the device's public `NodeId` derived from the stored secret key.
    async fn get_device_id(&self) -> StorageResult<NodeId>;

    /// Return the raw secret key bytes.
    ///
    /// Returns `Err(StorageError::Unauthorized)` on stores that cannot expose
    /// key material.
    async fn get_secret_key(&self) -> StorageResult<SecretKey>;

    /// Subscribe to ledger and typed local-metadata invalidations.
    /// A [`ServiceEvent::LedgerUpdated`] follows every successful save.
    /// Cross-process notifications may coalesce or duplicate; reload current state.
    fn subscribe(&self) -> broadcast::Receiver<ServiceEvent>;
}
// sirno:witness:unbill-storage:end
