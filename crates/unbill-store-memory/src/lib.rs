// In-memory LedgerStore implementation for unit tests.

use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;
use rand::TryRng as _;
use tokio::sync::broadcast;
use unbill_event::ServiceEvent;

use unbill_model::{Currency, LedgerId, LedgerMeta, NodeId, SecretKey, StorageError, Timestamp};
use unbill_model::{Invitation, LedgerDoc};
use unbill_storage::{LedgerStore, StorageResult as Result};

// sirno:witness:memory-store:begin
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
    labels: HashMap<String, String>,
    invitations: HashMap<String, Invitation>,
    secret: Option<[u8; 32]>,
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

    async fn list_device_labels(&self) -> Result<HashMap<String, String>> {
        Ok(self.inner.lock().unwrap().labels.clone())
    }
    async fn set_device_label(&self, node_id: &NodeId, label: Option<&str>) -> Result<()> {
        let mut inner = self.inner.lock().unwrap();
        match label {
            Some(label) => {
                inner.labels.insert(node_id.to_string(), label.to_owned());
            }
            None => {
                inner.labels.remove(&node_id.to_string());
            }
        }
        let _ = self.events.send(ServiceEvent::DeviceLabelsUpdated);
        Ok(())
    }
    async fn list_pending_invitations(&self) -> Result<Vec<Invitation>> {
        Ok(self
            .inner
            .lock()
            .unwrap()
            .invitations
            .values()
            .cloned()
            .collect())
    }
    async fn create_invitation(
        &self,
        ledger_id: LedgerId,
        created_by_device: &NodeId,
        created_at: Timestamp,
        expires_at: Timestamp,
    ) -> Result<Invitation> {
        let invitation = Invitation {
            token: unbill_model::InviteToken::generate(),
            ledger_id,
            created_by_device: created_by_device.clone(),
            created_at,
            expires_at,
        };
        self.inner
            .lock()
            .unwrap()
            .invitations
            .insert(invitation.token.to_string(), invitation.clone());
        let _ = self.events.send(ServiceEvent::PendingInvitationsUpdated);
        Ok(invitation)
    }
    async fn consume_invitation(&self, token: &str) -> Result<Option<Invitation>> {
        let invitation = self.inner.lock().unwrap().invitations.remove(token);
        if invitation.is_some() {
            let _ = self.events.send(ServiceEvent::PendingInvitationsUpdated);
        }
        Ok(invitation)
    }
    async fn create_secret_key(&self) -> Result<()> {
        let mut inner = self.inner.lock().unwrap();
        if inner.secret.is_none() {
            let mut bytes = [0; 32];
            rand::rngs::SysRng
                .try_fill_bytes(&mut bytes)
                .map_err(|e| StorageError::Io(std::io::Error::other(e.to_string())))?;
            inner.secret = Some(bytes);
            let _ = self.events.send(ServiceEvent::DeviceIdentityInitialized);
        }
        Ok(())
    }
    async fn is_device_initialized(&self) -> Result<bool> {
        Ok(self.inner.lock().unwrap().secret.is_some())
    }
    async fn get_device_id(&self) -> Result<NodeId> {
        let key = self.get_secret_key().await?;
        Ok(NodeId::new(
            iroh::SecretKey::from(*key.as_bytes()).public().to_string(),
        ))
    }
    async fn get_secret_key(&self) -> Result<SecretKey> {
        self.inner
            .lock()
            .unwrap()
            .secret
            .map(SecretKey::from_bytes)
            .ok_or_else(|| StorageError::Serialization("device not initialized".into()))
    }
}
// sirno:witness:memory-store:end
