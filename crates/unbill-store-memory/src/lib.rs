// In-memory LedgerStore implementation for unit tests.

use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard, PoisonError};

use async_trait::async_trait;
use rand::TryRng as _;
use tokio::sync::broadcast;
use unbill_event::ServiceEvent;

use unbill_model::{Invitation, LedgerDoc};
use unbill_model::{LedgerId, LedgerMeta, NodeId, SecretKey, StorageError, Timestamp};
use unbill_storage::{AccountRecord, LedgerStore, StorageResult as Result};

// sirno:witness:memory-store:begin
pub struct InMemoryStore {
    inner: Mutex<Inner>,
    events: broadcast::Sender<ServiceEvent>,
}

impl InMemoryStore {
    fn lock_inner(&self) -> MutexGuard<'_, Inner> {
        // Mutations replace or remove complete values, so poisoning needs no state repair.
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }
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
    accounts: HashMap<String, String>,
    sessions: HashMap<String, String>,
    grants: HashMap<String, Vec<LedgerId>>,
    ledgers: HashMap<String, StoredLedger>,
    invitations: HashMap<String, Invitation>,
    archived: HashMap<String, bool>,
    secret: Option<[u8; 32]>,
}

struct StoredLedger {
    meta: LedgerMeta,
    bytes: Vec<u8>,
}

fn metadata(doc: &LedgerDoc, updated_at: Timestamp) -> Result<LedgerMeta> {
    let ledger = doc
        .get_ledger()
        .map_err(|error| StorageError::Serialization(error.to_string()))?;
    Ok(LedgerMeta {
        archived: false,
        ledger_id: ledger.ledger_id,
        name: ledger.name,
        currency: ledger.currency,
        created_at: ledger.created_at,
        updated_at,
    })
}

#[async_trait]
impl LedgerStore for InMemoryStore {
    async fn create_account(&self, name: &str, password_hash: &str) -> Result<()> {
        let mut inner = self.lock_inner();
        if inner.accounts.contains_key(name) {
            return Err(
                std::io::Error::new(std::io::ErrorKind::AlreadyExists, "account exists").into(),
            );
        }
        inner
            .accounts
            .insert(name.to_owned(), password_hash.to_owned());
        Ok(())
    }
    async fn get_account(&self, name: &str) -> Result<Option<AccountRecord>> {
        Ok(self
            .lock_inner()
            .accounts
            .get(name)
            .map(|hash| AccountRecord {
                name: name.to_owned(),
                password_hash: hash.clone(),
            }))
    }
    async fn list_accounts(&self) -> Result<Vec<String>> {
        let mut names: Vec<_> = self.lock_inner().accounts.keys().cloned().collect();
        names.sort();
        Ok(names)
    }
    async fn set_account_password(&self, name: &str, password_hash: &str) -> Result<()> {
        let mut inner = self.lock_inner();
        let Some(hash) = inner.accounts.get_mut(name) else {
            return Err(
                std::io::Error::new(std::io::ErrorKind::NotFound, "account not found").into(),
            );
        };
        *hash = password_hash.to_owned();
        inner.sessions.retain(|_, account| account != name);
        Ok(())
    }
    async fn delete_account(&self, name: &str) -> Result<()> {
        let mut inner = self.lock_inner();
        inner.accounts.remove(name);
        inner.sessions.retain(|_, account| account != name);
        inner.grants.remove(name);
        Ok(())
    }
    async fn create_session(&self, token_hash: &str, account_name: &str) -> Result<()> {
        let mut inner = self.lock_inner();
        if !inner.accounts.contains_key(account_name) {
            return Err(StorageError::Unauthorized);
        }
        inner
            .sessions
            .insert(token_hash.to_owned(), account_name.to_owned());
        Ok(())
    }
    async fn resolve_session(&self, token_hash: &str) -> Result<Option<String>> {
        Ok(self.lock_inner().sessions.get(token_hash).cloned())
    }
    async fn delete_session(&self, token_hash: &str) -> Result<()> {
        self.lock_inner().sessions.remove(token_hash);
        Ok(())
    }
    async fn grant_ledger(&self, account_name: &str, ledger_id: LedgerId) -> Result<()> {
        let changed = {
            let mut inner = self.lock_inner();
            if !inner.accounts.contains_key(account_name) {
                return Err(StorageError::Unauthorized);
            }
            let grants = inner.grants.entry(account_name.to_owned()).or_default();
            if grants.contains(&ledger_id) {
                false
            } else {
                grants.push(ledger_id);
                true
            }
        };
        if changed {
            let _ = self.events.send(ServiceEvent::LedgerUpdated {
                ledger_id: ledger_id.to_string(),
            });
        }
        Ok(())
    }
    async fn revoke_ledger(&self, account_name: &str, ledger_id: LedgerId) -> Result<()> {
        if let Some(grants) = self.lock_inner().grants.get_mut(account_name) {
            grants.retain(|id| *id != ledger_id);
        }
        let _ = self.events.send(ServiceEvent::LedgerUpdated {
            ledger_id: ledger_id.to_string(),
        });
        Ok(())
    }
    async fn account_has_ledger(&self, account_name: &str, ledger_id: LedgerId) -> Result<bool> {
        Ok(self
            .lock_inner()
            .grants
            .get(account_name)
            .is_some_and(|grants| grants.contains(&ledger_id)))
    }
    async fn list_account_ledgers(&self, account_name: &str) -> Result<Vec<LedgerId>> {
        Ok(self
            .lock_inner()
            .grants
            .get(account_name)
            .cloned()
            .unwrap_or_default())
    }
    async fn save_ledger_meta(&self, meta: &LedgerMeta) -> Result<()> {
        let mut inner = self.lock_inner();
        let id = meta.ledger_id.to_string();
        match inner.ledgers.get_mut(&id) {
            Some(stored) if !stored.bytes.is_empty() => {
                stored.meta.updated_at = stored.meta.updated_at.max(meta.updated_at);
            }
            Some(stored) => stored.meta = meta.clone(),
            None => {
                inner.ledgers.insert(
                    id,
                    StoredLedger {
                        meta: meta.clone(),
                        bytes: vec![],
                    },
                );
            }
        }
        Ok(())
    }

    async fn set_ledger_archived(&self, ledger_id: LedgerId, archived: bool) -> Result<()> {
        let id = ledger_id.to_string();
        {
            let mut inner = self.lock_inner();
            if !inner.ledgers.contains_key(&id) {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("ledger not found: {id}"),
                )
                .into());
            }
            inner.archived.insert(id.clone(), archived);
        }
        let _ = self
            .events
            .send(ServiceEvent::LedgerUpdated { ledger_id: id });
        Ok(())
    }

    async fn list_ledgers(&self) -> Result<Vec<LedgerMeta>> {
        let inner = self.lock_inner();
        Ok(inner
            .ledgers
            .iter()
            .map(|(id, s)| {
                let mut meta = s.meta.clone();
                meta.archived = inner.archived.get(id).copied().unwrap_or(false);
                meta
            })
            .collect())
    }

    async fn load_ledger(&self, ledger_id: &str) -> Result<Option<LedgerDoc>> {
        let inner = self.lock_inner();
        match inner.ledgers.get(ledger_id) {
            None => Ok(None),
            Some(s) if s.bytes.is_empty() => Ok(None),
            Some(s) => LedgerDoc::from_bytes(&s.bytes)
                .map(Some)
                .map_err(|e| StorageError::Serialization(e.to_string())),
        }
    }

    async fn save_ledger(&self, ledger_id: &str, doc: &mut LedgerDoc) -> Result<()> {
        let mut meta = metadata(doc, Timestamp::now().map_err(std::io::Error::other)?)?;
        if meta.ledger_id.to_string() != ledger_id {
            return Err(StorageError::Serialization(
                "document ledger ID does not match storage key".into(),
            ));
        }
        let bytes = doc.save();
        {
            let mut inner = self.lock_inner();
            if let Some(stored) = inner.ledgers.get(ledger_id) {
                meta.updated_at = meta.updated_at.max(stored.meta.updated_at);
            }
            inner
                .ledgers
                .insert(ledger_id.to_owned(), StoredLedger { meta, bytes });
        }
        let _ = self.events.send(ServiceEvent::LedgerUpdated {
            ledger_id: ledger_id.to_owned(),
        });
        Ok(())
    }

    fn subscribe(&self) -> broadcast::Receiver<ServiceEvent> {
        self.events.subscribe()
    }

    async fn list_pending_invitations(&self) -> Result<Vec<Invitation>> {
        Ok(self.lock_inner().invitations.values().cloned().collect())
    }
    async fn create_invitation(
        &self,
        ledger_id: LedgerId,
        created_by_device: &NodeId,
        created_at: Timestamp,
        expires_at: Timestamp,
    ) -> Result<Invitation> {
        let invitation = Invitation {
            token: unbill_model::InviteToken::generate().map_err(std::io::Error::from)?,
            ledger_id,
            created_by_device: created_by_device.clone(),
            created_at,
            expires_at,
        };
        self.lock_inner()
            .invitations
            .insert(invitation.token.to_string(), invitation.clone());
        let _ = self.events.send(ServiceEvent::PendingInvitationsUpdated);
        Ok(invitation)
    }
    async fn consume_invitation(&self, token: &str) -> Result<Option<Invitation>> {
        let invitation = self.lock_inner().invitations.remove(token);
        if invitation.is_some() {
            let _ = self.events.send(ServiceEvent::PendingInvitationsUpdated);
        }
        Ok(invitation)
    }
    async fn create_secret_key(&self) -> Result<()> {
        let mut inner = self.lock_inner();
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
        Ok(self.lock_inner().secret.is_some())
    }
    async fn get_device_id(&self) -> Result<NodeId> {
        let key = self.get_secret_key().await?;
        Ok(NodeId::new(
            iroh::SecretKey::from(*key.as_bytes()).public().to_string(),
        ))
    }
    async fn get_secret_key(&self) -> Result<SecretKey> {
        self.lock_inner()
            .secret
            .map(SecretKey::from_bytes)
            .ok_or_else(|| StorageError::Serialization("device not initialized".into()))
    }
}
// sirno:witness:memory-store:end

#[cfg(test)]
mod tests {
    use super::*;
    use unbill_model::Currency;

    #[tokio::test]
    async fn poisoned_lock_preserves_ledger_and_accepts_updates() -> Result<()> {
        let store = InMemoryStore::default();
        let mut doc = LedgerDoc::new(
            LedgerId::from_u128(1),
            "Trip".into(),
            Currency::from_code("USD").unwrap(),
            Timestamp::from_millis(1),
        )
        .unwrap();
        let node_id = NodeId::new("peer".into());
        doc.add_device(
            unbill_model::NewDevice {
                node_id: node_id.clone(),
                label: unbill_model::DeviceLabel::new("Before".into()).unwrap(),
            },
            Timestamp::from_millis(1),
        )
        .unwrap();
        let id = LedgerId::from_u128(1).to_string();
        store.save_ledger(&id, &mut doc).await?;
        let panic = std::panic::catch_unwind(|| {
            let _guard = store.inner.lock().unwrap();
            panic!("simulate a panic while the store is locked");
        });
        assert!(panic.is_err());
        assert!(store.inner.is_poisoned());
        let mut doc = store.load_ledger(&id).await?.unwrap();
        assert_eq!(doc.list_devices().unwrap()[0].label.as_str(), "Before");
        doc.set_device_label(
            &node_id,
            unbill_model::DeviceLabel::new("After".into()).unwrap(),
        )
        .unwrap();
        store.save_ledger(&id, &mut doc).await?;
        assert_eq!(
            store
                .load_ledger(&id)
                .await?
                .unwrap()
                .list_devices()
                .unwrap()[0]
                .label
                .as_str(),
            "After"
        );
        Ok(())
    }
}
