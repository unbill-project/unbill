// Flat-file LedgerStore backed by `tokio::fs`.

pub mod path;
pub use path::{UNBILL_PATH, UnbillPath};

use std::{collections::HashMap, path::PathBuf};
use unbill_model::Invitation;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use rand::TryRng as _;
use tokio::sync::broadcast;
use unbill_event::ServiceEvent;
use unbill_model::LedgerDoc;
use unbill_model::{Currency, LedgerId, LedgerMeta, NodeId, SecretKey, StorageError, Timestamp};
use unbill_storage::{LedgerStore, StorageResult as Result};

// sirno:witness:fs-store:begin
pub struct FsStore {
    root: PathBuf,
    metadata_lock: tokio::sync::Mutex<()>,
    /// Holds `<root>/unbill.lock` open with an exclusive advisory lock for
    /// the lifetime of this store, preventing two processes from sharing the
    /// same data directory simultaneously, including on Mac Catalyst.
    /// Skipped on Android and non-Catalyst iOS.
    _lock: Option<std::fs::File>,
    events: broadcast::Sender<ServiceEvent>,
}

impl FsStore {
    /// Open the store at `root`, creating the directory if needed.
    ///
    /// On desktop (including Mac Catalyst), returns `Err` if another process
    /// already holds the directory lock. Android and non-Catalyst iOS skip it.
    pub fn open(root: PathBuf) -> std::io::Result<Self> {
        std::fs::create_dir_all(&root)?;
        let lock = if cfg!(any(
            target_os = "android",
            all(target_os = "ios", not(target_abi = "macabi"))
        )) {
            None
        } else {
            let lock_file = std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(false)
                .open(root.join("unbill.lock"))?;
            lock_file.try_lock().map_err(|e| match e {
                std::fs::TryLockError::WouldBlock => std::io::Error::new(
                    std::io::ErrorKind::WouldBlock,
                    format!("data directory is already in use: {}", root.display()),
                ),
                std::fs::TryLockError::Error(error) => error,
            })?;
            Some(lock_file)
        };
        let (events, _) = broadcast::channel(256);
        Ok(Self {
            root,
            metadata_lock: tokio::sync::Mutex::new(()),
            _lock: lock,
            events,
        })
    }

    fn ledger_dir(&self, ledger_id: &str) -> PathBuf {
        self.root.join("ledgers").join(ledger_id)
    }
}
// sirno:witness:fs-store:end

// ---------------------------------------------------------------------------
// JSON mirror of LedgerMeta — plain primitives, no domain-type imports needed
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize)]
struct MetaJson {
    ledger_id: String,
    name: String,
    currency: String,
    created_at_ms: i64,
    updated_at_ms: i64,
}

impl MetaJson {
    fn from_meta(meta: &LedgerMeta) -> Self {
        Self {
            ledger_id: meta.ledger_id.to_string(),
            name: meta.name.clone(),
            currency: meta.currency.code().to_owned(),
            created_at_ms: meta.created_at.as_millis(),
            updated_at_ms: meta.updated_at.as_millis(),
        }
    }

    fn into_ledger_meta(self) -> std::result::Result<LedgerMeta, String> {
        let ledger_id = LedgerId::from_string(&self.ledger_id).map_err(|e| e.to_string())?;
        let currency = Currency::from_code(&self.currency)
            .ok_or_else(|| format!("unknown currency code {:?}", self.currency))?;
        Ok(LedgerMeta {
            ledger_id,
            name: self.name,
            currency,
            created_at: Timestamp::from_millis(self.created_at_ms),
            updated_at: Timestamp::from_millis(self.updated_at_ms),
        })
    }
}

// ---------------------------------------------------------------------------
// LedgerStore impl
// ---------------------------------------------------------------------------

// sirno:witness:fs-store:begin
#[async_trait]
impl LedgerStore for FsStore {
    async fn save_ledger_meta(&self, meta: &LedgerMeta) -> Result<()> {
        let dir = self.ledger_dir(&meta.ledger_id.to_string());
        tokio::fs::create_dir_all(&dir).await?;
        let json = serde_json::to_string_pretty(&MetaJson::from_meta(meta))
            .map_err(|e| StorageError::Serialization(e.to_string()))?;
        atomic_write(dir.join("meta.json"), json.as_bytes()).await
    }

    async fn list_ledgers(&self) -> Result<Vec<LedgerMeta>> {
        let ledgers_dir = self.root.join("ledgers");
        let mut entries = match tokio::fs::read_dir(&ledgers_dir).await {
            Ok(e) => e,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
            Err(e) => return Err(e.into()),
        };
        let mut metas = Vec::new();
        while let Some(entry) = entries.next_entry().await? {
            if !entry.file_type().await?.is_dir() {
                continue;
            }
            let meta_path = entry.path().join("meta.json");
            match tokio::fs::read(&meta_path).await {
                Ok(bytes) => {
                    match serde_json::from_slice::<MetaJson>(&bytes)
                        .map_err(|e| e.to_string())
                        .and_then(|m| m.into_ledger_meta())
                    {
                        Ok(meta) => metas.push(meta),
                        Err(e) => tracing::warn!("skipping {:?}: {e}", entry.path()),
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    tracing::warn!("skipping {:?}: no meta.json", entry.path());
                }
                Err(e) => return Err(e.into()),
            }
        }
        Ok(metas)
    }

    async fn load_ledger(&self, ledger_id: &str) -> Result<Option<LedgerDoc>> {
        match tokio::fs::read(self.ledger_dir(ledger_id).join("ledger.bin")).await {
            Ok(bytes) => LedgerDoc::from_bytes(&bytes)
                .map(Some)
                .map_err(|e| StorageError::Serialization(e.to_string())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    async fn save_ledger(&self, ledger_id: &str, doc: &mut LedgerDoc) -> Result<()> {
        let dir = self.ledger_dir(ledger_id);
        tokio::fs::create_dir_all(&dir).await?;
        atomic_write(dir.join("ledger.bin"), &doc.save()).await?;
        if self
            .events
            .send(ServiceEvent::LedgerUpdated {
                ledger_id: ledger_id.to_owned(),
            })
            .is_err()
        {
            tracing::warn!(ledger_id, "LedgerUpdated event dropped: no subscribers");
        }
        Ok(())
    }

    fn subscribe(&self) -> broadcast::Receiver<ServiceEvent> {
        self.events.subscribe()
    }

    async fn list_device_labels(&self) -> Result<HashMap<String, String>> {
        read_json(self.root.join("device_labels.json")).await
    }
    async fn set_device_label(&self, node_id: &NodeId, label: Option<&str>) -> Result<()> {
        let _guard = self.metadata_lock.lock().await;
        let mut labels = self.list_device_labels().await?;
        match label {
            Some(label) => {
                labels.insert(node_id.to_string(), label.to_owned());
            }
            None => {
                labels.remove(&node_id.to_string());
            }
        }
        write_json(self.root.join("device_labels.json"), &labels).await
    }
    async fn list_pending_invitations(&self) -> Result<Vec<Invitation>> {
        let map: HashMap<String, Invitation> =
            read_json(self.root.join("pending_invitations.json")).await?;
        Ok(map.into_values().collect())
    }
    async fn save_invitation(&self, invitation: &Invitation) -> Result<()> {
        let _guard = self.metadata_lock.lock().await;
        let path = self.root.join("pending_invitations.json");
        let mut map: HashMap<String, Invitation> = read_json(path.clone()).await?;
        map.insert(invitation.token.to_string(), invitation.clone());
        write_json(path, &map).await
    }
    async fn consume_invitation(&self, token: &str) -> Result<Option<Invitation>> {
        let _guard = self.metadata_lock.lock().await;
        let path = self.root.join("pending_invitations.json");
        let mut map: HashMap<String, Invitation> = read_json(path.clone()).await?;
        let invitation = map.remove(token);
        if invitation.is_some() {
            write_json(path, &map).await?;
        }
        Ok(invitation)
    }

    async fn create_secret_key(&self) -> Result<()> {
        let _guard = self.metadata_lock.lock().await;
        if self.is_device_initialized().await? {
            return Ok(());
        }
        let mut arr = [0u8; 32];
        rand::rngs::SysRng
            .try_fill_bytes(&mut arr)
            .expect("system RNG should generate device keys");
        atomic_write(self.root.join("device_key.bin"), &arr).await
    }

    async fn is_device_initialized(&self) -> Result<bool> {
        Ok(tokio::fs::try_exists(self.root.join("device_key.bin")).await?)
    }

    async fn get_device_id(&self) -> Result<NodeId> {
        let key = self.get_secret_key().await?;
        Ok(NodeId::new(
            iroh::SecretKey::from(*key.as_bytes()).public().to_string(),
        ))
    }
    async fn get_secret_key(&self) -> Result<SecretKey> {
        let bytes = tokio::fs::read(self.root.join("device_key.bin"))
            .await
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    StorageError::Serialization("device not initialized".into())
                } else {
                    e.into()
                }
            })?;
        let bytes: [u8; 32] = bytes
            .try_into()
            .map_err(|_| StorageError::Serialization("device_key.bin: wrong length".into()))?;
        Ok(SecretKey::from_bytes(bytes))
    }
}
// sirno:witness:fs-store:end

// sirno:witness:unbill-storage:begin
async fn atomic_write(path: PathBuf, bytes: &[u8]) -> Result<()> {
    let tmp = path.with_extension("tmp");
    tokio::fs::write(&tmp, bytes).await?;
    tokio::fs::rename(&tmp, &path).await?;
    Ok(())
}
// sirno:witness:unbill-storage:end

async fn read_json<T: serde::de::DeserializeOwned + Default>(path: PathBuf) -> Result<T> {
    match tokio::fs::read(path).await {
        Ok(bytes) => {
            serde_json::from_slice(&bytes).map_err(|e| StorageError::Serialization(e.to_string()))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(T::default()),
        Err(e) => Err(e.into()),
    }
}
async fn write_json<T: Serialize>(path: PathBuf, value: &T) -> Result<()> {
    let bytes =
        serde_json::to_vec(value).map_err(|e| StorageError::Serialization(e.to_string()))?;
    atomic_write(path, &bytes).await
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use unbill_model::{LedgerDoc, Timestamp};

    fn make_meta(name: &str) -> LedgerMeta {
        LedgerMeta {
            ledger_id: LedgerId::from_u128(1),
            name: name.to_owned(),
            currency: Currency::from_code("USD").unwrap(),
            created_at: Timestamp::from_millis(1_000),
            updated_at: Timestamp::from_millis(2_000),
        }
    }

    fn make_doc(name: &str) -> LedgerDoc {
        LedgerDoc::new(
            LedgerId::from_u128(1),
            name.to_owned(),
            Currency::from_code("USD").unwrap(),
            Timestamp::from_millis(1_000),
        )
        .unwrap()
    }

    #[test]
    fn second_open_on_same_dir_fails() {
        let dir = tempfile::tempdir().unwrap();
        let _first = FsStore::open(dir.path().to_path_buf()).unwrap();
        let second = FsStore::open(dir.path().to_path_buf());
        assert_eq!(
            second.err().expect("second open must fail").kind(),
            std::io::ErrorKind::WouldBlock
        );
        drop(_first);
        assert!(
            FsStore::open(dir.path().to_path_buf()).is_ok(),
            "dropping the owner must release the lock"
        );
    }

    #[tokio::test]
    async fn test_save_and_list_ledger_meta() {
        let dir = tempfile::tempdir().unwrap();
        let store = FsStore::open(dir.path().to_path_buf()).unwrap();
        assert!(store.list_ledgers().await.unwrap().is_empty());
        let meta = make_meta("Groceries");
        store.save_ledger_meta(&meta).await.unwrap();
        let listed = store.list_ledgers().await.unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].name, "Groceries");
    }

    #[tokio::test]
    async fn test_save_and_load_ledger_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let store = FsStore::open(dir.path().to_path_buf()).unwrap();
        let id = LedgerId::from_u128(1).to_string();
        assert!(store.load_ledger(&id).await.unwrap().is_none());
        let mut doc = make_doc("Test");
        store.save_ledger(&id, &mut doc).await.unwrap();
        let loaded = store.load_ledger(&id).await.unwrap().unwrap();
        assert_eq!(loaded.get_ledger().unwrap().name, "Test");
    }
}
