// UnbillService: top-level facade consumed by CLI and Tauri.

use std::collections::HashMap;
use std::sync::Arc;

use dashmap::DashMap;
use rand::RngCore as _;
use tokio::sync::{broadcast, Mutex};

use crate::doc::LedgerDoc;
use crate::error::{Result, UnbillError};
use crate::model::{
    BillAmendment, Currency, EffectiveBill, Invitation, LedgerMeta, Member, NewBill, NewMember,
    NodeId, Timestamp, Ulid,
};
use crate::settlement;
use crate::storage::LedgerStore;

pub struct UnbillService {
    store: Arc<dyn LedgerStore>,
    device_id: NodeId,
    /// Eagerly loaded in-memory ledger documents, keyed by ledger ID string.
    ledgers: DashMap<String, Arc<Mutex<LedgerDoc>>>,
    /// Pending join invitations. In-memory only — never persisted.
    pending_invitations: std::sync::Mutex<HashMap<String, Invitation>>,
    events: broadcast::Sender<ServiceEvent>,
}

#[derive(Clone, Debug)]
pub enum ServiceEvent {
    LedgerUpdated {
        ledger_id: String,
    },
    PeerConnected {
        ledger_id: String,
        peer: String,
    },
    PeerDisconnected {
        ledger_id: String,
        peer: String,
    },
    SyncError {
        ledger_id: String,
        peer: String,
        error: String,
    },
}

impl UnbillService {
    /// Open the service: load or create the device key, then eagerly load all
    /// stored ledgers into memory.
    pub async fn open(store: Arc<dyn LedgerStore>) -> Result<Arc<Self>> {
        let device_id = load_or_create_device_key(&*store).await?;

        let metas = store.list_ledgers().await?;
        let ledgers: DashMap<String, Arc<Mutex<LedgerDoc>>> = DashMap::new();
        for meta in metas {
            let id = meta.ledger_id.to_string();
            let bytes = store.load_ledger_bytes(&id).await?;
            let doc = LedgerDoc::from_bytes(&bytes).map_err(|e| UnbillError::Other(e))?;
            ledgers.insert(id, Arc::new(Mutex::new(doc)));
        }

        let (events, _) = broadcast::channel(256);
        Ok(Arc::new(Self {
            store,
            device_id,
            ledgers,
            pending_invitations: std::sync::Mutex::new(HashMap::new()),
            events,
        }))
    }

    // -----------------------------------------------------------------------
    // Ledger lifecycle
    // -----------------------------------------------------------------------

    pub async fn create_ledger(&self, name: String, currency: String) -> Result<String> {
        let currency = Currency::from_code(&currency).ok_or_else(|| {
            UnbillError::Other(anyhow::anyhow!("unknown currency code: {currency}"))
        })?;
        let ledger_id = Ulid::new();
        let now = Timestamp::now();

        let mut doc = LedgerDoc::new(ledger_id, name.clone(), currency.clone(), now)?;
        let bytes = doc.save();

        let meta = LedgerMeta {
            ledger_id,
            name,
            currency,
            created_at: now,
            updated_at: now,
        };
        let id = ledger_id.to_string();
        self.store.save_ledger_meta(&meta).await?;
        self.store.save_ledger_bytes(&id, &bytes).await?;
        self.ledgers.insert(id.clone(), Arc::new(Mutex::new(doc)));

        Ok(id)
    }

    pub async fn list_ledgers(&self) -> Result<Vec<LedgerMeta>> {
        Ok(self.store.list_ledgers().await?)
    }

    pub async fn delete_ledger(&self, ledger_id: &str) -> Result<()> {
        self.ledgers.remove(ledger_id);
        self.store.delete_ledger(ledger_id).await?;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Bills
    // -----------------------------------------------------------------------

    pub async fn add_bill(&self, ledger_id: &str, input: NewBill) -> Result<String> {
        let doc_mutex = self.get_doc(ledger_id)?;
        let mut doc = doc_mutex.lock().await;
        let bill_id = doc.add_bill(input, self.device_id, Timestamp::now())?;
        let bytes = doc.save();
        drop(doc);

        self.store.save_ledger_bytes(ledger_id, &bytes).await?;
        self.touch_meta(ledger_id).await?;
        Ok(bill_id.to_string())
    }

    pub async fn amend_bill(
        &self,
        ledger_id: &str,
        bill_id: &str,
        input: BillAmendment,
    ) -> Result<()> {
        let bill_ulid = parse_ulid(bill_id)?;
        let doc_mutex = self.get_doc(ledger_id)?;
        let mut doc = doc_mutex.lock().await;
        doc.amend_bill(&bill_ulid, input, Timestamp::now())?;
        let bytes = doc.save();
        drop(doc);

        self.store.save_ledger_bytes(ledger_id, &bytes).await?;
        self.touch_meta(ledger_id).await?;
        Ok(())
    }

    pub async fn delete_bill(&self, ledger_id: &str, bill_id: &str) -> Result<()> {
        let bill_ulid = parse_ulid(bill_id)?;
        let doc_mutex = self.get_doc(ledger_id)?;
        let mut doc = doc_mutex.lock().await;
        doc.delete_bill(&bill_ulid)?;
        let bytes = doc.save();
        drop(doc);

        self.store.save_ledger_bytes(ledger_id, &bytes).await?;
        self.touch_meta(ledger_id).await?;
        Ok(())
    }

    pub async fn restore_bill(&self, ledger_id: &str, bill_id: &str) -> Result<()> {
        let bill_ulid = parse_ulid(bill_id)?;
        let doc_mutex = self.get_doc(ledger_id)?;
        let mut doc = doc_mutex.lock().await;
        doc.restore_bill(&bill_ulid)?;
        let bytes = doc.save();
        drop(doc);

        self.store.save_ledger_bytes(ledger_id, &bytes).await?;
        self.touch_meta(ledger_id).await?;
        Ok(())
    }

    pub async fn list_bills(&self, ledger_id: &str) -> Result<Vec<EffectiveBill>> {
        let doc_mutex = self.get_doc(ledger_id)?;
        let doc = doc_mutex.lock().await;
        doc.list_bills()
    }

    // -----------------------------------------------------------------------
    // Members
    // -----------------------------------------------------------------------

    pub async fn add_member(&self, ledger_id: &str, input: NewMember) -> Result<()> {
        let doc_mutex = self.get_doc(ledger_id)?;
        let mut doc = doc_mutex.lock().await;
        doc.add_member(input, Timestamp::now())?;
        let bytes = doc.save();
        drop(doc);

        self.store.save_ledger_bytes(ledger_id, &bytes).await?;
        self.touch_meta(ledger_id).await?;
        Ok(())
    }

    pub async fn remove_member(&self, ledger_id: &str, user_id: &str) -> Result<()> {
        let user_ulid = parse_ulid(user_id)?;
        let doc_mutex = self.get_doc(ledger_id)?;
        let mut doc = doc_mutex.lock().await;
        doc.remove_member(&user_ulid)?;
        let bytes = doc.save();
        drop(doc);

        self.store.save_ledger_bytes(ledger_id, &bytes).await?;
        self.touch_meta(ledger_id).await?;
        Ok(())
    }

    pub async fn list_members(&self, ledger_id: &str) -> Result<Vec<Member>> {
        let doc_mutex = self.get_doc(ledger_id)?;
        let doc = doc_mutex.lock().await;
        doc.list_members()
    }

    // -----------------------------------------------------------------------
    // Settlement
    // -----------------------------------------------------------------------

    pub async fn compute_settlement(&self, ledger_id: &str) -> Result<settlement::Settlement> {
        let doc_mutex = self.get_doc(ledger_id)?;
        let doc = doc_mutex.lock().await;
        let members = doc.list_members()?;
        let bills = doc.list_bills()?;
        Ok(settlement::compute(&members, &bills))
    }

    // -----------------------------------------------------------------------
    // Events
    // -----------------------------------------------------------------------

    pub fn device_id(&self) -> NodeId {
        self.device_id
    }

    pub fn subscribe(&self) -> broadcast::Receiver<ServiceEvent> {
        self.events.subscribe()
    }

    // -----------------------------------------------------------------------
    // Internals
    // -----------------------------------------------------------------------

    fn get_doc(&self, ledger_id: &str) -> Result<Arc<Mutex<LedgerDoc>>> {
        self.ledgers
            .get(ledger_id)
            .map(|r| Arc::clone(&*r))
            .ok_or_else(|| UnbillError::LedgerNotFound(ledger_id.to_string()))
    }

    /// Update `updated_at` in the stored metadata for a ledger.
    async fn touch_meta(&self, ledger_id: &str) -> Result<()> {
        let mut metas = self.store.list_ledgers().await?;
        if let Some(meta) = metas
            .iter_mut()
            .find(|m| m.ledger_id.to_string() == ledger_id)
        {
            meta.updated_at = Timestamp::now();
            self.store.save_ledger_meta(meta).await?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

async fn load_or_create_device_key(store: &dyn LedgerStore) -> Result<NodeId> {
    if let Some(bytes) = store.load_device_meta("device_key.bin").await? {
        let arr: [u8; 32] = bytes
            .try_into()
            .map_err(|_| UnbillError::Other(anyhow::anyhow!("device_key.bin: wrong length")))?;
        let secret = iroh::SecretKey::from(arr);
        Ok(NodeId::from_node_id(secret.public()))
    } else {
        let mut arr = [0u8; 32];
        rand::rngs::OsRng.fill_bytes(&mut arr);
        let secret = iroh::SecretKey::from(arr);
        store.save_device_meta("device_key.bin", &arr).await?;
        Ok(NodeId::from_node_id(secret.public()))
    }
}

fn parse_ulid(s: &str) -> Result<Ulid> {
    Ulid::from_string(s).map_err(|e| UnbillError::Other(anyhow::anyhow!("invalid ULID {s:?}: {e}")))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Share;
    use crate::storage::InMemoryStore;

    fn mem_store() -> Arc<dyn LedgerStore> {
        Arc::new(InMemoryStore::default())
    }

    async fn open() -> Arc<UnbillService> {
        UnbillService::open(mem_store()).await.unwrap()
    }

    fn usd() -> &'static str {
        "USD"
    }

    fn two_way_bill(desc: &str, amount_cents: i64, ledger_id: &str) -> (String, NewBill) {
        let _ = ledger_id;
        let bill = NewBill {
            payer_user_id: Ulid::from_u128(1),
            amount_cents,
            description: desc.to_owned(),
            shares: vec![
                Share { user_id: Ulid::from_u128(1), shares: 1 },
                Share { user_id: Ulid::from_u128(2), shares: 1 },
            ],
        };
        (ledger_id.to_owned(), bill)
    }

    async fn seed_members(svc: &UnbillService, ledger_id: &str) {
        for (n, name) in [(1u128, "Alice"), (2, "Bob")] {
            svc.add_member(
                ledger_id,
                NewMember {
                    user_id: Ulid::from_u128(n),
                    display_name: name.into(),
                    added_by: Ulid::from_u128(1),
                },
            )
            .await
            .unwrap();
        }
    }

    // --- create / list / delete ledger ---

    #[tokio::test]
    async fn test_create_ledger_appears_in_list() {
        let svc = open().await;
        let id = svc
            .create_ledger("Household".into(), usd().into())
            .await
            .unwrap();
        let ledgers = svc.list_ledgers().await.unwrap();
        assert_eq!(ledgers.len(), 1);
        assert_eq!(ledgers[0].ledger_id.to_string(), id);
        assert_eq!(ledgers[0].name, "Household");
        assert_eq!(ledgers[0].currency.code(), "USD");
    }

    #[tokio::test]
    async fn test_delete_ledger_removes_it() {
        let svc = open().await;
        let id = svc
            .create_ledger("Trip".into(), usd().into())
            .await
            .unwrap();
        svc.delete_ledger(&id).await.unwrap();
        assert!(svc.list_ledgers().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_unknown_ledger_returns_not_found() {
        let svc = open().await;
        let result = svc.list_bills("00000000000000000000000000").await;
        assert!(matches!(result, Err(UnbillError::LedgerNotFound(_))));
    }

    #[tokio::test]
    async fn test_invalid_currency_returns_error() {
        let svc = open().await;
        let result = svc.create_ledger("Bad".into(), "ZZZ".into()).await;
        assert!(result.is_err());
    }

    // --- bills ---

    #[tokio::test]
    async fn test_add_bill_and_list() {
        let svc = open().await;
        let lid = svc
            .create_ledger("Test".into(), usd().into())
            .await
            .unwrap();
        seed_members(&svc, &lid).await;
        let (_, bill) = two_way_bill("Dinner", 6000, &lid);
        let bill_id = svc.add_bill(&lid, bill).await.unwrap();

        let bills = svc.list_bills(&lid).await.unwrap();
        assert_eq!(bills.len(), 1);
        assert_eq!(bills[0].id.to_string(), bill_id);
        assert_eq!(bills[0].amount_cents, 6000);
    }

    #[tokio::test]
    async fn test_amend_bill_updates_amount() {
        let svc = open().await;
        let lid = svc
            .create_ledger("Test".into(), usd().into())
            .await
            .unwrap();
        seed_members(&svc, &lid).await;
        let (_, bill) = two_way_bill("Lunch", 3000, &lid);
        let bill_id = svc.add_bill(&lid, bill).await.unwrap();

        svc.amend_bill(
            &lid,
            &bill_id,
            BillAmendment {
                new_amount_cents: Some(4000),
                new_description: None,
                new_shares: None,
                author_user_id: Ulid::from_u128(1),
                reason: None,
            },
        )
        .await
        .unwrap();

        let bills = svc.list_bills(&lid).await.unwrap();
        assert_eq!(bills[0].amount_cents, 4000);
        assert!(bills[0].was_amended);
    }

    #[tokio::test]
    async fn test_delete_and_restore_bill() {
        let svc = open().await;
        let lid = svc
            .create_ledger("Test".into(), usd().into())
            .await
            .unwrap();
        seed_members(&svc, &lid).await;
        let (_, bill) = two_way_bill("Coffee", 500, &lid);
        let bill_id = svc.add_bill(&lid, bill).await.unwrap();

        svc.delete_bill(&lid, &bill_id).await.unwrap();
        assert!(svc.list_bills(&lid).await.unwrap()[0].is_deleted);

        svc.restore_bill(&lid, &bill_id).await.unwrap();
        assert!(!svc.list_bills(&lid).await.unwrap()[0].is_deleted);
    }

    // --- settlement ---

    #[tokio::test]
    async fn test_compute_settlement_no_bills_is_empty() {
        let svc = open().await;
        let lid = svc
            .create_ledger("Empty".into(), usd().into())
            .await
            .unwrap();
        let s = svc.compute_settlement(&lid).await.unwrap();
        assert!(s.transactions.is_empty());
    }

    // --- persistence round-trip ---

    #[tokio::test]
    async fn test_ledger_survives_service_restart() {
        let store = mem_store();
        let lid = {
            let svc = UnbillService::open(Arc::clone(&store)).await.unwrap();
            let lid = svc
                .create_ledger("Persistent".into(), usd().into())
                .await
                .unwrap();
            seed_members(&svc, &lid).await;
            let (_, bill) = two_way_bill("Rent", 120000, &lid);
            svc.add_bill(&lid, bill).await.unwrap();
            lid
        };
        // Re-open with the same store (simulates a restart).
        let svc2 = UnbillService::open(Arc::clone(&store)).await.unwrap();
        let bills = svc2.list_bills(&lid).await.unwrap();
        assert_eq!(bills.len(), 1);
        assert_eq!(bills[0].amount_cents, 120000);
    }

    #[tokio::test]
    async fn test_device_key_stable_across_restarts() {
        let store = mem_store();
        let svc1 = UnbillService::open(Arc::clone(&store)).await.unwrap();
        let svc2 = UnbillService::open(Arc::clone(&store)).await.unwrap();
        assert_eq!(svc1.device_id, svc2.device_id);
    }
}
