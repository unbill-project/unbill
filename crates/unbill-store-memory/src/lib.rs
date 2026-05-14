// In-memory LedgerStore implementation for unit tests.

use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;
use rand::TryRng as _;

use unbill_model::{Currency, LedgerId, LedgerMeta, NodeId, SecretKey, StorageError, Timestamp};
use unbill_storage::{LedgerDoc, LedgerStore, StorageResult as Result};

#[derive(Default)]
pub struct InMemoryStore {
    inner: Mutex<Inner>,
}

#[derive(Default)]
struct Inner {
    ledgers: HashMap<String, StoredLedger>,
    device_meta: HashMap<String, Vec<u8>>,
}

struct StoredLedger {
    meta: LedgerMeta,
    bytes: Vec<u8>,
}

#[async_trait]
impl LedgerStore for InMemoryStore {
    async fn save_ledger_meta(&self, meta: &LedgerMeta) -> Result<()> {
        let mut inner = self.inner.lock().unwrap();
        let id = meta.ledger_id.to_string();
        inner
            .ledgers
            .entry(id)
            .and_modify(|s| s.meta = meta.clone())
            .or_insert_with(|| StoredLedger {
                meta: meta.clone(),
                bytes: vec![],
            });
        Ok(())
    }

    async fn list_ledgers(&self) -> Result<Vec<LedgerMeta>> {
        let inner = self.inner.lock().unwrap();
        Ok(inner.ledgers.values().map(|s| s.meta.clone()).collect())
    }

    async fn load_ledger(&self, ledger_id: &str) -> Result<Option<LedgerDoc>> {
        let inner = self.inner.lock().unwrap();
        match inner.ledgers.get(ledger_id) {
            None => Ok(None),
            Some(s) if s.bytes.is_empty() => Ok(None),
            Some(s) => LedgerDoc::from_bytes(&s.bytes)
                .map(Some)
                .map_err(|e| StorageError::Serialization(e.to_string())),
        }
    }

    async fn save_ledger(&self, ledger_id: &str, doc: &mut LedgerDoc) -> Result<()> {
        let mut inner = self.inner.lock().unwrap();
        let bytes = doc.save();
        inner
            .ledgers
            .entry(ledger_id.to_owned())
            .and_modify(|s| s.bytes = bytes.clone())
            .or_insert_with(|| StoredLedger {
                meta: LedgerMeta {
                    ledger_id: LedgerId::from_u128(0),
                    name: String::new(),
                    currency: Currency::from_code("USD").unwrap(),
                    created_at: Timestamp::from_millis(0),
                    updated_at: Timestamp::from_millis(0),
                },
                bytes,
            });
        Ok(())
    }

    async fn delete_ledger(&self, ledger_id: &str) -> Result<()> {
        let mut inner = self.inner.lock().unwrap();
        inner.ledgers.remove(ledger_id);
        Ok(())
    }

    async fn load_device_meta(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let inner = self.inner.lock().unwrap();
        Ok(inner.device_meta.get(key).cloned())
    }

    async fn save_device_meta(&self, key: &str, value: &[u8]) -> Result<()> {
        let mut inner = self.inner.lock().unwrap();
        inner.device_meta.insert(key.to_owned(), value.to_vec());
        Ok(())
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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use unbill_storage::LockableStore;
    use unbill_store_memory_guard::LockedStore;

    #[tokio::test]
    async fn test_lock_guard_derefs_to_ledger_store() {
        let store = LockedStore::new(InMemoryStore::default());
        let guard = store.lock().await.unwrap();
        let ledgers = guard.list_ledgers().await.unwrap();
        assert!(ledgers.is_empty());
    }

    #[tokio::test]
    async fn test_lock_is_exclusive() {
        let store = LockedStore::new(InMemoryStore::default());
        let store2 = store.clone();

        let guard = store.lock().await.unwrap();

        let task = tokio::spawn(async move { store2.lock().await.unwrap() });

        // Yield so the spawned task gets a chance to attempt the lock.
        tokio::task::yield_now().await;
        assert!(
            !task.is_finished(),
            "second lock should block while first guard is held"
        );

        drop(guard);

        tokio::time::timeout(std::time::Duration::from_secs(1), task)
            .await
            .expect("lock should be released within timeout")
            .unwrap();
    }
}
