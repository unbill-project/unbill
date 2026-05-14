// Flat-file LedgerStore backed by `tokio::fs`.

pub mod path;
pub use path::{UNBILL_PATH, UnbillPath};

use std::path::PathBuf;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use rand::TryRng as _;
use unbill_model::{Currency, LedgerId, LedgerMeta, NodeId, SecretKey, StorageError, Timestamp};
use unbill_storage::{LedgerDoc, LedgerStore, StorageResult as Result};

#[derive(Clone)]
pub struct FsStore {
    root: PathBuf,
}

impl FsStore {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn ledger_dir(&self, ledger_id: &str) -> PathBuf {
        self.root.join("ledgers").join(ledger_id)
    }
}

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
        atomic_write(dir.join("ledger.bin"), &doc.save()).await
    }

    async fn delete_ledger(&self, ledger_id: &str) -> Result<()> {
        match tokio::fs::remove_dir_all(self.ledger_dir(ledger_id)).await {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.into()),
        }
    }

    async fn load_device_meta(&self, key: &str) -> Result<Option<Vec<u8>>> {
        match tokio::fs::read(self.root.join(key)).await {
            Ok(b) => Ok(Some(b)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    async fn save_device_meta(&self, key: &str, value: &[u8]) -> Result<()> {
        tokio::fs::create_dir_all(&self.root).await?;
        atomic_write(self.root.join(key), value).await
    }

    async fn create_secret_key(&self) -> Result<()> {
        if self.load_device_meta("device_key.bin").await?.is_some() {
            return Ok(());
        }
        let mut arr = [0u8; 32];
        rand::rngs::SysRng
            .try_fill_bytes(&mut arr)
            .expect("system RNG should generate device keys");
        self.save_device_meta("device_key.bin", &arr).await
    }

    async fn is_device_initialized(&self) -> Result<bool> {
        Ok(self.load_device_meta("device_key.bin").await?.is_some())
    }

    async fn get_device_id(&self) -> Result<NodeId> {
        let bytes = self
            .load_device_meta("device_key.bin")
            .await?
            .ok_or_else(|| StorageError::Serialization("device not initialized".into()))?;
        let arr: [u8; 32] = bytes
            .try_into()
            .map_err(|_| StorageError::Serialization("device_key.bin: wrong length".into()))?;
        let secret = iroh::SecretKey::from(arr);
        Ok(NodeId::new(secret.public().to_string()))
    }

    async fn get_secret_key(&self) -> Result<SecretKey> {
        let bytes = self
            .load_device_meta("device_key.bin")
            .await?
            .ok_or_else(|| StorageError::Serialization("device not initialized".into()))?;
        let arr: [u8; 32] = bytes
            .try_into()
            .map_err(|_| StorageError::Serialization("device_key.bin: wrong length".into()))?;
        Ok(SecretKey::from_bytes(arr))
    }
}

async fn atomic_write(path: PathBuf, bytes: &[u8]) -> Result<()> {
    let tmp = path.with_extension("tmp");
    tokio::fs::write(&tmp, bytes).await?;
    tokio::fs::rename(&tmp, &path).await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use unbill_model::Timestamp;
    use unbill_storage::{LedgerDoc, LockableStore};

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

    #[tokio::test]
    async fn test_save_and_list_ledger_meta() {
        let dir = tempfile::tempdir().unwrap();
        let store = FsStore::new(dir.path().to_path_buf());
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
        let store = FsStore::new(dir.path().to_path_buf());
        let id = LedgerId::from_u128(1).to_string();
        assert!(store.load_ledger(&id).await.unwrap().is_none());
        let mut doc = make_doc("Test");
        store.save_ledger(&id, &mut doc).await.unwrap();
        let loaded = store.load_ledger(&id).await.unwrap().unwrap();
        assert_eq!(loaded.get_ledger().unwrap().name, "Test");
    }

    #[tokio::test]
    async fn test_delete_ledger() {
        let dir = tempfile::tempdir().unwrap();
        let store = FsStore::new(dir.path().to_path_buf());
        let meta = make_meta("ToDelete");
        store.save_ledger_meta(&meta).await.unwrap();
        let id = meta.ledger_id.to_string();
        let mut doc = make_doc("ToDelete");
        store.save_ledger(&id, &mut doc).await.unwrap();
        assert_eq!(store.list_ledgers().await.unwrap().len(), 1);
        store.delete_ledger(&id).await.unwrap();
        assert!(store.list_ledgers().await.unwrap().is_empty());
        store.delete_ledger(&id).await.unwrap(); // idempotent
    }

    #[tokio::test]
    async fn test_device_meta_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let store = FsStore::new(dir.path().to_path_buf());
        assert!(
            store
                .load_device_meta("device_key.bin")
                .await
                .unwrap()
                .is_none()
        );
        store
            .save_device_meta("device_key.bin", b"secret")
            .await
            .unwrap();
        let loaded = store.load_device_meta("device_key.bin").await.unwrap();
        assert_eq!(loaded.as_deref(), Some(b"secret".as_ref()));
    }

    #[tokio::test]
    async fn test_lock_creates_sentinel_file() {
        let dir = tempfile::tempdir().unwrap();
        let locked = unbill_store_fs_guard::FileLockedStore::new(
            FsStore::new(dir.path().to_path_buf()),
            dir.path().join(".lock"),
        );
        let _guard = locked.lock().await.unwrap();
        assert!(dir.path().join(".lock").exists());
    }

    #[tokio::test]
    async fn test_lock_sentinel_persists_after_drop() {
        let dir = tempfile::tempdir().unwrap();
        let locked = unbill_store_fs_guard::FileLockedStore::new(
            FsStore::new(dir.path().to_path_buf()),
            dir.path().join(".lock"),
        );
        let guard = locked.lock().await.unwrap();
        drop(guard);
        assert!(dir.path().join(".lock").exists());
    }

    #[tokio::test]
    async fn test_lock_is_exclusive() {
        let dir = tempfile::tempdir().unwrap();
        let locked = unbill_store_fs_guard::FileLockedStore::new(
            FsStore::new(dir.path().to_path_buf()),
            dir.path().join(".lock"),
        );
        let guard = locked.lock().await.unwrap();

        let lock_path = dir.path().join(".lock");
        let f = std::fs::OpenOptions::new()
            .write(true)
            .open(&lock_path)
            .unwrap();
        assert!(
            f.try_lock().is_err(),
            "second fd should not acquire the lock while the guard is held"
        );

        drop(guard);

        f.try_lock()
            .expect("second fd should acquire the lock after the guard is dropped");
    }

    #[tokio::test]
    async fn test_lock_guard_derefs_to_ledger_store() {
        let dir = tempfile::tempdir().unwrap();
        let locked = unbill_store_fs_guard::FileLockedStore::new(
            FsStore::new(dir.path().to_path_buf()),
            dir.path().join(".lock"),
        );
        let guard = locked.lock().await.unwrap();
        let ledgers = guard.list_ledgers().await.unwrap();
        assert!(ledgers.is_empty());
    }
}
