use async_trait::async_trait;
use tokio::sync::broadcast;
use unbill_event::ServiceEvent;

use unbill_model::{LedgerMeta, NodeId, SecretKey, StorageError};

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
    /// authoritative merged state after the call.
    async fn save_ledger(&self, ledger_id: &str, doc: &mut LedgerDoc) -> StorageResult<()>;

    async fn load_device_meta(&self, key: &str) -> StorageResult<Option<Vec<u8>>>;
    async fn save_device_meta(&self, key: &str, value: &[u8]) -> StorageResult<()>;

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

    /// Subscribe to ledger change events.  A [`ServiceEvent::LedgerUpdated`]
    /// is sent every time [`save_ledger`] completes successfully.
    fn subscribe(&self) -> broadcast::Receiver<ServiceEvent>;
}
// sirno:witness:unbill-storage:end
