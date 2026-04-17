use async_trait::async_trait;

use crate::model::LedgerMeta;

pub type Result<T> = std::result::Result<T, crate::error::StorageError>;

#[async_trait]
pub trait LedgerStore: Send + Sync {
    /// Create or update the per-ledger metadata cache.
    async fn save_ledger_meta(&self, meta: &LedgerMeta) -> Result<()>;
    async fn list_ledgers(&self) -> Result<Vec<LedgerMeta>>;
    /// Load the full Automerge snapshot bytes for a ledger.
    /// Returns empty `Vec` if the ledger has never been saved.
    async fn load_ledger_bytes(&self, ledger_id: &str) -> Result<Vec<u8>>;
    /// Atomically overwrite the stored snapshot with new bytes.
    async fn save_ledger_bytes(&self, ledger_id: &str, bytes: &[u8]) -> Result<()>;
    async fn delete_ledger(&self, ledger_id: &str) -> Result<()>;

    async fn load_device_meta(&self, key: &str) -> Result<Option<Vec<u8>>>;
    async fn save_device_meta(&self, key: &str, value: &[u8]) -> Result<()>;
}
