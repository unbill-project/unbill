// In-memory LedgerStore implementation for unit tests.

use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;

use crate::model::LedgerMeta;

use super::traits::{LedgerStore, Result};

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
            .or_insert_with(|| StoredLedger { meta: meta.clone(), bytes: vec![] });
        Ok(())
    }

    async fn list_ledgers(&self) -> Result<Vec<LedgerMeta>> {
        let inner = self.inner.lock().unwrap();
        Ok(inner.ledgers.values().map(|s| s.meta.clone()).collect())
    }

    async fn load_ledger_bytes(&self, ledger_id: &str) -> Result<Vec<u8>> {
        let inner = self.inner.lock().unwrap();
        Ok(inner.ledgers.get(ledger_id).map(|s| s.bytes.clone()).unwrap_or_default())
    }

    async fn save_ledger_bytes(&self, ledger_id: &str, bytes: &[u8]) -> Result<()> {
        let mut inner = self.inner.lock().unwrap();
        if let Some(s) = inner.ledgers.get_mut(ledger_id) {
            s.bytes = bytes.to_vec();
        }
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
}
