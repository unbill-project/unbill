// In-memory LedgerStore implementation for unit tests.

use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;
use rand::TryRng as _;
use tokio::sync::broadcast;
use unbill_event::ServiceEvent;

use unbill_model::{Currency, LedgerId, LedgerMeta, NodeId, SecretKey, StorageError, Timestamp};
use unbill_storage::{LedgerDoc, LedgerStore, StorageResult as Result};

pub struct InMemoryStore {
    inner: Mutex<Inner>,
    events: broadcast::Sender<ServiceEvent>,
}

impl Default for InMemoryStore {
    fn default() -> Self {
        let (events, _) = broadcast::channel(256);
        Self {
            inner: Mutex::new(Inner::default()),
            events,
        }
    }
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
        let bytes = doc.save();
        {
            let mut inner = self.inner.lock().unwrap();
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
        }
        let _ = self.events.send(ServiceEvent::LedgerUpdated {
            ledger_id: ledger_id.to_owned(),
        });
        Ok(())
    }

    fn subscribe(&self) -> broadcast::Receiver<ServiceEvent> {
        self.events.subscribe()
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
