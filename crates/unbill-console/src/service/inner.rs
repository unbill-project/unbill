// UnbillConsole: top-level facade consumed by CLI and TUI.

use std::collections::HashMap;
use std::sync::Arc;

use automerge::sync;
use tokio::sync::{Mutex, broadcast};
use unbill_asymmetric_channel::{AsymChannel, AsymChannelEvent};

use crate::conflict::{self, ConflictGroup};
use crate::error::{Result, UnbillError};
use garde::validate::Validate as _;

use crate::model::{
    BillId, Currency, Device, EffectiveBills, LedgerId, LedgerMeta, NewBill, NewDevice, NewLedger,
    NewUser, NewUserName, NodeId, StorageError, Timestamp, User, UserId,
};

use crate::settlement;
use unbill_event::ServiceEvent;
use unbill_storage::LedgerDoc;

// sirno:witness:console-service:begin
pub struct UnbillConsole {
    channel: Arc<dyn AsymChannel>,
    events: broadcast::Sender<ServiceEvent>,
    cache: Arc<Mutex<HashMap<LedgerId, LedgerDoc>>>,
}
// sirno:witness:console-service:end

impl UnbillConsole {
    // sirno:witness:console-service:begin
    /// Wire the console to a channel, prime the ledger cache, and start the
    /// event bridge.
    pub async fn open(channel: Arc<dyn AsymChannel>) -> Arc<Self> {
        let (events, _) = broadcast::channel(256);
        let cache: Arc<Mutex<HashMap<LedgerId, LedgerDoc>>> = Arc::new(Mutex::new(HashMap::new()));

        // Prime the cache with every ledger the device already knows about.
        if let Ok(metas) = channel.list_ledgers().await {
            let mut map = cache.lock().await;
            for meta in metas {
                let mut doc = LedgerDoc::empty();
                if sync_doc(&*channel, meta.ledger_id, &mut doc).await.is_ok() && !doc.is_empty() {
                    map.insert(meta.ledger_id, doc);
                }
            }
        }

        let console = Arc::new(Self {
            channel,
            events,
            cache,
        });

        // Bridge AsymChannelEvents → ServiceEvents and refresh the cache.
        // The async body is identical on both targets; only the spawn call differs.
        {
            let mut ch_rx = console.channel.subscribe_to_server();
            let events_tx = console.events.clone();
            let cache = Arc::clone(&console.cache);
            let channel = Arc::clone(&console.channel);
            let fut = async move {
                while let Ok(evt) = ch_rx.recv().await {
                    match evt {
                        AsymChannelEvent::LedgerUpdated { ledger_id } => {
                            let mut doc = cache
                                .lock()
                                .await
                                .remove(&ledger_id)
                                .unwrap_or_else(LedgerDoc::empty);
                            if sync_doc(&*channel, ledger_id, &mut doc).await.is_ok()
                                && !doc.is_empty()
                            {
                                cache.lock().await.insert(ledger_id, doc);
                            }
                            if events_tx
                                .send(ServiceEvent::LedgerUpdated {
                                    ledger_id: ledger_id.to_string(),
                                })
                                .is_err()
                            {
                                tracing::warn!("LedgerUpdated event dropped: no subscribers");
                            }
                        }
                    }
                }
            };
            #[cfg(not(target_arch = "wasm32"))]
            tokio::spawn(fut);
            #[cfg(target_arch = "wasm32")]
            wasm_bindgen_futures::spawn_local(fut);
        }

        console
    }
    // sirno:witness:console-service:end

    // -----------------------------------------------------------------------
    // Ledger lifecycle
    // -----------------------------------------------------------------------

    // sirno:witness:console-service:begin
    pub async fn create_ledger(&self, input: NewLedger) -> Result<LedgerId> {
        input
            .validate()
            .map_err(|e| UnbillError::Validation(e.to_string()))?;
        let ledger_id = LedgerId::new();
        let now = Timestamp::now();
        let mut doc = LedgerDoc::new(ledger_id, input.name.clone(), input.currency, now)?;
        doc.add_device(
            NewDevice {
                node_id: self.channel.device_id(),
            },
            now,
        )?;
        let meta = LedgerMeta {
            ledger_id,
            name: input.name,
            currency: input.currency,
            created_at: now,
            updated_at: now,
        };
        self.channel.save_ledger_meta(&meta).await?;
        sync_doc(&*self.channel, ledger_id, &mut doc).await?;
        self.put_doc(ledger_id, doc).await;
        Ok(ledger_id)
    }

    pub async fn list_ledgers(&self) -> Result<Vec<LedgerMeta>> {
        self.channel.list_ledgers().await
    }
    // sirno:witness:console-service:end

    // -----------------------------------------------------------------------
    // Bills
    // -----------------------------------------------------------------------

    // sirno:witness:bill-amendment:begin
    pub async fn add_bill(&self, ledger_id: LedgerId, input: NewBill) -> Result<BillId> {
        input
            .validate()
            .map_err(|e| UnbillError::Validation(e.to_string()))?;
        let mut doc = self.take_doc(ledger_id).await?;
        let bill_id = doc.add_bill(input, self.channel.device_id(), Timestamp::now())?;
        sync_doc(&*self.channel, ledger_id, &mut doc).await?;
        self.put_doc(ledger_id, doc).await;
        self.touch_meta(ledger_id).await?;
        if self
            .events
            .send(ServiceEvent::LedgerUpdated {
                ledger_id: ledger_id.to_string(),
            })
            .is_err()
        {
            tracing::warn!("LedgerUpdated event dropped: no subscribers");
        }
        Ok(bill_id)
    }
    // sirno:witness:bill-amendment:end

    pub async fn list_bills(&self, ledger_id: LedgerId) -> Result<EffectiveBills> {
        let doc = self.take_doc(ledger_id).await?;
        let result = doc.list_bills();
        self.put_doc(ledger_id, doc).await;
        result
    }

    // -----------------------------------------------------------------------
    // Users
    // -----------------------------------------------------------------------

    // sirno:witness:users-and-devices:begin
    pub async fn add_user(&self, ledger_id: LedgerId, input: NewUser) -> Result<()> {
        input
            .validate()
            .map_err(|e| UnbillError::Validation(e.to_string()))?;
        let mut doc = self.take_doc(ledger_id).await?;
        doc.add_user(input, Timestamp::now())?;
        sync_doc(&*self.channel, ledger_id, &mut doc).await?;
        self.put_doc(ledger_id, doc).await;
        self.touch_meta(ledger_id).await?;
        if self
            .events
            .send(ServiceEvent::LedgerUpdated {
                ledger_id: ledger_id.to_string(),
            })
            .is_err()
        {
            tracing::warn!("LedgerUpdated event dropped: no subscribers");
        }
        Ok(())
    }

    pub async fn list_users(&self, ledger_id: LedgerId) -> Result<Vec<User>> {
        let doc = self.take_doc(ledger_id).await?;
        let result = doc.list_users();
        self.put_doc(ledger_id, doc).await;
        result
    }

    /// Create a brand-new user, add them to the ledger, and return the created `User`.
    pub async fn create_user(&self, ledger_id: LedgerId, input: NewUserName) -> Result<User> {
        input
            .validate()
            .map_err(|e| UnbillError::Validation(e.to_string()))?;
        let user_id = UserId::new();
        let now = Timestamp::now();
        let mut doc = self.take_doc(ledger_id).await?;
        doc.add_user(
            NewUser {
                user_id,
                display_name: input.display_name.clone(),
            },
            now,
        )?;
        sync_doc(&*self.channel, ledger_id, &mut doc).await?;
        self.put_doc(ledger_id, doc).await;
        self.touch_meta(ledger_id).await?;
        if self
            .events
            .send(ServiceEvent::LedgerUpdated {
                ledger_id: ledger_id.to_string(),
            })
            .is_err()
        {
            tracing::warn!("LedgerUpdated event dropped: no subscribers");
        }
        Ok(User {
            user_id,
            display_name: input.display_name,
            added_at: now,
        })
    }

    /// Return all unique users across every ledger on this device, deduplicated by `user_id`.
    pub async fn list_all_users(&self) -> Result<Vec<User>> {
        let mut seen = std::collections::HashSet::new();
        let mut result = Vec::new();
        for meta in self.channel.list_ledgers().await? {
            let doc = self.take_doc(meta.ledger_id).await?;
            let users = doc.list_users()?;
            self.put_doc(meta.ledger_id, doc).await;
            for user in users {
                if seen.insert(user.user_id) {
                    result.push(user);
                }
            }
        }
        Ok(result)
    }
    // sirno:witness:users-and-devices:end

    // -----------------------------------------------------------------------
    // Devices
    // -----------------------------------------------------------------------

    // sirno:witness:users-and-devices:begin
    pub async fn add_device(&self, ledger_id: LedgerId, input: NewDevice) -> Result<()> {
        let mut doc = self.take_doc(ledger_id).await?;
        doc.add_device(input, Timestamp::now())?;
        sync_doc(&*self.channel, ledger_id, &mut doc).await?;
        self.put_doc(ledger_id, doc).await;
        self.touch_meta(ledger_id).await?;
        Ok(())
    }

    pub async fn list_devices(&self, ledger_id: LedgerId) -> Result<Vec<Device>> {
        let doc = self.take_doc(ledger_id).await?;
        let result = doc.list_devices();
        self.put_doc(ledger_id, doc).await;
        result
    }

    pub async fn list_device_labels(&self) -> Result<HashMap<String, String>> {
        match self.channel.load_device_meta("device_labels.json").await? {
            None => Ok(HashMap::new()),
            Some(bytes) => serde_json::from_slice(&bytes).map_err(|e| {
                UnbillError::Storage(StorageError::Serialization(format!(
                    "device_labels.json: {e}"
                )))
            }),
        }
    }

    pub async fn set_device_label(&self, node_id: NodeId, label: String) -> Result<()> {
        let mut labels = self.list_device_labels().await?;
        let key = node_id.to_string();
        let trimmed = label.trim();
        if trimmed.is_empty() {
            labels.remove(&key);
        } else {
            labels.insert(key, trimmed.to_owned());
        }
        let bytes = serde_json::to_vec(&labels).map_err(|e| {
            UnbillError::Storage(StorageError::Serialization(format!(
                "serialize device_labels: {e}"
            )))
        })?;
        self.channel
            .save_device_meta("device_labels.json", bytes)
            .await
    }
    // sirno:witness:users-and-devices:end

    // -----------------------------------------------------------------------
    // Settlement
    // -----------------------------------------------------------------------

    // sirno:witness:settlement:begin
    /// Compute net settlement for a user across all ledgers they participate in,
    /// grouped by currency.
    pub async fn compute_settlement_for_user(
        &self,
        user_id: UserId,
    ) -> Result<Vec<settlement::Settlement>> {
        let mut by_currency: std::collections::HashMap<
            Currency,
            std::collections::HashMap<UserId, i64>,
        > = std::collections::HashMap::new();

        for meta in self.channel.list_ledgers().await? {
            let doc = self.take_doc(meta.ledger_id).await?;
            let users = doc.list_users()?;
            if users.iter().any(|user| user.user_id == user_id) {
                let currency = doc.get_ledger()?.currency;
                let bills = doc.list_bills()?;
                self.put_doc(meta.ledger_id, doc).await;
                let balances = by_currency.entry(currency).or_default();
                settlement::accumulate_balances(&users, &bills, balances);
            } else {
                self.put_doc(meta.ledger_id, doc).await;
            }
        }

        let mut results = Vec::new();
        for (currency, balances) in by_currency {
            let full = settlement::compute_from_balances(currency, balances);
            let transactions: Vec<_> = full
                .transactions
                .into_iter()
                .filter(|t| t.from_user_id == user_id || t.to_user_id == user_id)
                .collect();
            if !transactions.is_empty() {
                results.push(settlement::Settlement {
                    currency,
                    transactions,
                });
            }
        }
        Ok(results)
    }

    /// Compute settlement for a single ledger.
    pub async fn settle_ledger(
        &self,
        ledger_id: LedgerId,
    ) -> crate::error::Result<settlement::Settlement> {
        let doc = self.take_doc(ledger_id).await?;
        let ledger = doc.get_ledger()?;
        let result = settlement::compute_settlement(&ledger);
        self.put_doc(ledger_id, doc).await;
        Ok(result)
    }
    // sirno:witness:settlement:end

    // -----------------------------------------------------------------------
    // Conflict detection
    // -----------------------------------------------------------------------

    // sirno:witness:conflict-detection:begin
    pub async fn detect_conflicts(&self, ledger_id: LedgerId) -> Result<Vec<ConflictGroup>> {
        let doc = self.take_doc(ledger_id).await?;
        let all_bills = doc.list_all_bills()?;
        self.put_doc(ledger_id, doc).await;
        Ok(conflict::detect(&all_bills))
    }
    // sirno:witness:conflict-detection:end

    // -----------------------------------------------------------------------
    // Metadata pass-throughs (used by server router)
    // -----------------------------------------------------------------------

    // sirno:witness:asymmetric-channel:begin
    pub async fn save_ledger_meta(&self, meta: &LedgerMeta) -> Result<()> {
        self.channel.save_ledger_meta(meta).await
    }

    pub async fn load_device_meta(&self, key: &str) -> Result<Option<Vec<u8>>> {
        self.channel.load_device_meta(key).await
    }

    pub async fn save_device_meta(&self, key: &str, bytes: Vec<u8>) -> Result<()> {
        self.channel.save_device_meta(key, bytes).await
    }

    /// One round of Automerge sync — device side. Used by the HTTP server.
    pub async fn asym_sync(&self, ledger_id: LedgerId, bytes: Vec<u8>) -> Result<Option<Vec<u8>>> {
        self.channel.asym_sync(ledger_id, bytes).await
    }
    // sirno:witness:asymmetric-channel:end

    // -----------------------------------------------------------------------
    // Invitations and sync
    // -----------------------------------------------------------------------

    // sirno:witness:sync-behavior:begin
    pub async fn create_invitation(&self, ledger_id: LedgerId) -> Result<String> {
        self.channel.create_invitation(ledger_id).await
    }

    pub async fn join_ledger(&self, url: &str, label: Option<String>) -> Result<()> {
        self.channel.join_ledger(url.to_owned(), label).await
    }

    pub async fn sync_once(&self, peer: NodeId) -> Result<()> {
        self.channel.trigger_peer_sync(peer).await
    }
    // sirno:witness:sync-behavior:end

    // -----------------------------------------------------------------------
    // Events and identity
    // -----------------------------------------------------------------------

    pub fn device_id(&self) -> NodeId {
        self.channel.device_id()
    }

    pub fn subscribe(&self) -> broadcast::Receiver<ServiceEvent> {
        self.events.subscribe()
    }

    // -----------------------------------------------------------------------
    // Internals
    // -----------------------------------------------------------------------

    /// Remove the doc for `ledger_id` from the cache and return it.
    /// Falls back to a full sync from the device if the cache is cold.
    // sirno:witness:console-service:begin
    async fn take_doc(&self, ledger_id: LedgerId) -> Result<LedgerDoc> {
        if let Some(doc) = self.cache.lock().await.remove(&ledger_id) {
            return Ok(doc);
        }
        let mut doc = LedgerDoc::empty();
        sync_doc(&*self.channel, ledger_id, &mut doc).await?;
        if doc.is_empty() {
            return Err(UnbillError::LedgerNotFound(ledger_id.to_string()));
        }
        Ok(doc)
    }

    /// Insert `doc` back into the cache under `ledger_id`.
    async fn put_doc(&self, ledger_id: LedgerId, doc: LedgerDoc) {
        self.cache.lock().await.insert(ledger_id, doc);
    }

    /// Update `updated_at` in the stored metadata for a ledger.
    async fn touch_meta(&self, ledger_id: LedgerId) -> Result<()> {
        let mut metas = self.channel.list_ledgers().await?;
        if let Some(meta) = metas.iter_mut().find(|m| m.ledger_id == ledger_id) {
            meta.updated_at = Timestamp::now();
            self.channel.save_ledger_meta(meta).await?;
        }
        Ok(())
    }
    // sirno:witness:console-service:end
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Run a full Automerge sync loop between the console's local `doc` and the
/// device via `channel`. Works for both initial fetch (empty doc) and push
/// (doc already contains local mutations).
// sirno:witness:asymmetric-channel:begin
async fn sync_doc(
    channel: &dyn AsymChannel,
    ledger_id: LedgerId,
    doc: &mut LedgerDoc,
) -> Result<()> {
    let mut sync_state = sync::State::new();
    while let Some(msg) = doc.generate_sync_message(&mut sync_state) {
        match channel.asym_sync(ledger_id, msg.encode()).await? {
            Some(resp_bytes) => {
                let resp = sync::Message::decode(&resp_bytes)
                    .map_err(|e| UnbillError::Automerge(e.to_string()))?;
                doc.receive_sync_message(&mut sync_state, resp)
                    .map_err(|e| UnbillError::Automerge(e.to_string()))?;
            }
            None => break,
        }
    }
    Ok(())
}
// sirno:witness:asymmetric-channel:end

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{BillId, NewLedger, Share};
    use unbill_asymmetric_channel::AsymChannelEvent;
    use unbill_storage::LedgerStore;
    use unbill_store_memory::InMemoryStore;

    // -----------------------------------------------------------------------
    // MockAsymChannel — in-process device backed by InMemoryStore
    // -----------------------------------------------------------------------

    struct MockAsymChannel {
        store: Arc<dyn LedgerStore>,
        device_id: NodeId,
        events: broadcast::Sender<AsymChannelEvent>,
    }

    impl MockAsymChannel {
        async fn new(store: Arc<dyn LedgerStore>) -> Arc<Self> {
            store.create_secret_key().await.unwrap();
            let device_id = store.get_device_id().await.unwrap();
            let (events, _) = broadcast::channel(1);
            Arc::new(Self {
                store,
                device_id,
                events,
            })
        }
    }

    #[async_trait::async_trait]
    impl AsymChannel for MockAsymChannel {
        fn device_id(&self) -> NodeId {
            self.device_id.clone()
        }

        async fn create_invitation(&self, _: LedgerId) -> Result<String> {
            unimplemented!()
        }
        async fn join_ledger(&self, _: String, _: Option<String>) -> Result<()> {
            unimplemented!()
        }
        async fn trigger_peer_sync(&self, _: NodeId) -> Result<()> {
            unimplemented!()
        }

        async fn asym_sync(&self, ledger_id: LedgerId, bytes: Vec<u8>) -> Result<Option<Vec<u8>>> {
            use automerge::sync::Message;
            let id_str = ledger_id.to_string();
            let client_msg =
                Message::decode(&bytes).map_err(|e| UnbillError::Automerge(e.to_string()))?;
            let mut doc = self
                .store
                .load_ledger(&id_str)
                .await?
                .unwrap_or_else(LedgerDoc::empty);
            let mut sync_state = automerge::sync::State::new();
            doc.receive_sync_message(&mut sync_state, client_msg)
                .map_err(|e| UnbillError::Automerge(e.to_string()))?;
            if !doc.is_empty() {
                self.store.save_ledger(&id_str, &mut doc).await?;
            }
            Ok(doc
                .generate_sync_message(&mut sync_state)
                .map(|m| m.encode()))
        }

        async fn list_ledgers(&self) -> Result<Vec<LedgerMeta>> {
            Ok(self.store.list_ledgers().await?)
        }
        async fn save_ledger_meta(&self, meta: &LedgerMeta) -> Result<()> {
            Ok(self.store.save_ledger_meta(meta).await?)
        }
        async fn load_device_meta(&self, key: &str) -> Result<Option<Vec<u8>>> {
            Ok(self.store.load_device_meta(key).await?)
        }
        async fn save_device_meta(&self, key: &str, bytes: Vec<u8>) -> Result<()> {
            Ok(self.store.save_device_meta(key, &bytes).await?)
        }
        fn subscribe_to_server(&self) -> broadcast::Receiver<AsymChannelEvent> {
            self.events.subscribe()
        }
    }

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    fn mem_store() -> Arc<dyn LedgerStore> {
        Arc::new(InMemoryStore::default())
    }

    async fn open() -> Arc<UnbillConsole> {
        let store = mem_store();
        let channel = MockAsymChannel::new(store).await;
        UnbillConsole::open(channel).await
    }

    fn usd() -> Currency {
        Currency::from_code("USD").unwrap()
    }

    fn two_way_bill(amount_cents: i64) -> NewBill {
        NewBill {
            amount_cents,
            description: "Dinner".to_owned(),
            payers: vec![Share {
                user_id: UserId::from_u128(1),
                shares: 1,
            }],
            payees: vec![
                Share {
                    user_id: UserId::from_u128(1),
                    shares: 1,
                },
                Share {
                    user_id: UserId::from_u128(2),
                    shares: 1,
                },
            ],
            prev: vec![],
        }
    }

    async fn seed_users(svc: &UnbillConsole, ledger_id: LedgerId) {
        for (n, name) in [(1u128, "Alice"), (2, "Bob")] {
            svc.add_user(
                ledger_id,
                NewUser {
                    user_id: UserId::from_u128(n),
                    display_name: name.into(),
                },
            )
            .await
            .unwrap();
        }
    }

    // --- create / list ledger ---

    #[tokio::test]
    async fn test_create_ledger_appears_in_list() {
        let svc = open().await;
        let id = svc
            .create_ledger(NewLedger {
                name: "Household".into(),
                currency: usd(),
            })
            .await
            .unwrap();
        let ledgers = svc.list_ledgers().await.unwrap();
        assert_eq!(ledgers.len(), 1);
        assert_eq!(ledgers[0].ledger_id, id);
        assert_eq!(ledgers[0].name, "Household");
        assert_eq!(ledgers[0].currency.code(), "USD");
    }

    #[tokio::test]
    async fn test_unknown_ledger_returns_not_found() {
        let svc = open().await;
        let result = svc
            .list_bills(LedgerId::from_string("00000000000000000000000000").unwrap())
            .await;
        assert!(matches!(result, Err(UnbillError::LedgerNotFound(_))));
    }

    // --- bills ---

    #[tokio::test]
    async fn test_add_bill_and_list() {
        let svc = open().await;
        let lid = svc
            .create_ledger(NewLedger {
                name: "Test".into(),
                currency: usd(),
            })
            .await
            .unwrap();
        seed_users(&svc, lid).await;
        let bill_id = svc.add_bill(lid, two_way_bill(6000)).await.unwrap();

        let bills = svc.list_bills(lid).await.unwrap();
        assert_eq!(bills.0.len(), 1);
        assert_eq!(bills.0[0].id, bill_id);
        assert_eq!(bills.0[0].amount_cents, 6000);
    }

    #[tokio::test]
    async fn test_amend_bill_supersedes_original() {
        let svc = open().await;
        let lid = svc
            .create_ledger(NewLedger {
                name: "Test".into(),
                currency: usd(),
            })
            .await
            .unwrap();
        seed_users(&svc, lid).await;
        let original_id = svc.add_bill(lid, two_way_bill(3000)).await.unwrap();

        let amendment_id = svc
            .add_bill(
                lid,
                NewBill {
                    amount_cents: 4000,
                    description: "Lunch".into(),
                    payers: vec![Share {
                        user_id: UserId::from_u128(1),
                        shares: 1,
                    }],
                    payees: vec![
                        Share {
                            user_id: UserId::from_u128(1),
                            shares: 1,
                        },
                        Share {
                            user_id: UserId::from_u128(2),
                            shares: 1,
                        },
                    ],
                    prev: vec![original_id],
                },
            )
            .await
            .unwrap();

        let bills = svc.list_bills(lid).await.unwrap();
        assert_eq!(bills.0.len(), 1, "original should be superseded");
        assert_eq!(bills.0[0].id, amendment_id);
        assert_eq!(bills.0[0].amount_cents, 4000);
    }

    // --- settlement ---

    #[tokio::test]
    async fn test_compute_settlement_no_bills_is_empty() {
        let svc = open().await;
        let lid = svc
            .create_ledger(NewLedger {
                name: "Empty".into(),
                currency: usd(),
            })
            .await
            .unwrap();
        seed_users(&svc, lid).await;
        let alice = UserId::from_u128(1);
        let s = svc.compute_settlement_for_user(alice).await.unwrap();
        assert!(s.iter().all(|g| g.transactions.is_empty()));
    }

    #[tokio::test]
    async fn test_compute_settlement_cross_ledger() {
        let svc = open().await;

        let lid1 = svc
            .create_ledger(NewLedger {
                name: "L1".into(),
                currency: usd(),
            })
            .await
            .unwrap();
        seed_users(&svc, lid1).await;
        svc.add_bill(lid1, two_way_bill(6000)).await.unwrap();

        let lid2 = svc
            .create_ledger(NewLedger {
                name: "L2".into(),
                currency: usd(),
            })
            .await
            .unwrap();
        seed_users(&svc, lid2).await;
        let bob_pays = NewBill {
            amount_cents: 2000,
            description: "Utilities".into(),
            payers: vec![Share {
                user_id: UserId::from_u128(2),
                shares: 1,
            }],
            payees: vec![
                Share {
                    user_id: UserId::from_u128(1),
                    shares: 1,
                },
                Share {
                    user_id: UserId::from_u128(2),
                    shares: 1,
                },
            ],
            prev: vec![],
        };
        svc.add_bill(lid2, bob_pays).await.unwrap();

        let alice = UserId::from_u128(1);
        let settlements = svc.compute_settlement_for_user(alice).await.unwrap();
        assert_eq!(settlements.len(), 1);
        let s = &settlements[0];
        assert_eq!(s.currency.code(), "USD");
        assert_eq!(s.transactions.len(), 1);
        assert_eq!(s.transactions[0].amount_cents, 2000);
        assert_eq!(s.transactions[0].from_user_id, UserId::from_u128(2));
        assert_eq!(s.transactions[0].to_user_id, UserId::from_u128(1));
    }

    #[tokio::test]
    async fn test_settlement_groups_separate_currencies() {
        let svc = open().await;

        let lid_usd = svc
            .create_ledger(NewLedger {
                name: "USD ledger".into(),
                currency: usd(),
            })
            .await
            .unwrap();
        seed_users(&svc, lid_usd).await;
        svc.add_bill(lid_usd, two_way_bill(6000)).await.unwrap();

        let lid_eur = svc
            .create_ledger(NewLedger {
                name: "EUR ledger".into(),
                currency: Currency::from_code("EUR").unwrap(),
            })
            .await
            .unwrap();
        seed_users(&svc, lid_eur).await;
        svc.add_bill(lid_eur, two_way_bill(4000)).await.unwrap();

        let alice = UserId::from_u128(1);
        let mut settlements = svc.compute_settlement_for_user(alice).await.unwrap();

        assert_eq!(settlements.len(), 2);
        settlements.sort_by_key(|s| s.currency.code());

        let eur = &settlements[0];
        assert_eq!(eur.currency.code(), "EUR");
        assert_eq!(eur.transactions.len(), 1);
        assert_eq!(eur.transactions[0].amount_cents, 2000);

        let usd = &settlements[1];
        assert_eq!(usd.currency.code(), "USD");
        assert_eq!(usd.transactions.len(), 1);
        assert_eq!(usd.transactions[0].amount_cents, 3000);
    }

    // --- persistence round-trip ---

    #[tokio::test]
    async fn test_ledger_survives_service_restart() {
        let store = mem_store();
        let lid = {
            let channel = MockAsymChannel::new(Arc::clone(&store)).await;
            let svc = UnbillConsole::open(channel).await;
            let lid = svc
                .create_ledger(NewLedger {
                    name: "Persistent".into(),
                    currency: usd(),
                })
                .await
                .unwrap();
            seed_users(&svc, lid).await;
            svc.add_bill(lid, two_way_bill(120000)).await.unwrap();
            lid
        };
        let channel2 = MockAsymChannel::new(Arc::clone(&store)).await;
        let svc2 = UnbillConsole::open(channel2).await;
        let bills = svc2.list_bills(lid).await.unwrap();
        assert_eq!(bills.0.len(), 1);
        assert_eq!(bills.0[0].amount_cents, 120000);
    }

    #[tokio::test]
    async fn test_device_key_stable_across_restarts() {
        let store = mem_store();
        let channel1 = MockAsymChannel::new(Arc::clone(&store)).await;
        let channel2 = MockAsymChannel::new(Arc::clone(&store)).await;
        let svc1 = UnbillConsole::open(channel1).await;
        let svc2 = UnbillConsole::open(channel2).await;
        assert_eq!(svc1.device_id(), svc2.device_id());
    }

    #[tokio::test]
    async fn test_device_labels_survive_restart() {
        let store = mem_store();
        let peer = NodeId::from_seed(9);
        {
            let channel = MockAsymChannel::new(Arc::clone(&store)).await;
            let svc = UnbillConsole::open(channel).await;
            svc.set_device_label(peer.clone(), "Kitchen iPad".into())
                .await
                .unwrap();
        }
        let channel2 = MockAsymChannel::new(Arc::clone(&store)).await;
        let svc2 = UnbillConsole::open(channel2).await;
        let labels = svc2.list_device_labels().await.unwrap();
        assert_eq!(
            labels.get(&peer.to_string()).map(String::as_str),
            Some("Kitchen iPad")
        );
    }

    // --- create_user / list_all_users ---

    #[tokio::test]
    async fn test_create_user_appears_in_ledger() {
        let svc = open().await;
        let lid = svc
            .create_ledger(NewLedger {
                name: "Trip".into(),
                currency: usd(),
            })
            .await
            .unwrap();
        let user = svc
            .create_user(
                lid,
                NewUserName {
                    display_name: "Alice".into(),
                },
            )
            .await
            .unwrap();
        let list = svc.list_users(lid).await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].user_id, user.user_id);
        assert_eq!(list[0].display_name, "Alice");
    }

    #[tokio::test]
    async fn test_list_all_users_aggregates_across_ledgers() {
        let svc = open().await;
        let lid1 = svc
            .create_ledger(NewLedger {
                name: "L1".into(),
                currency: usd(),
            })
            .await
            .unwrap();
        let lid2 = svc
            .create_ledger(NewLedger {
                name: "L2".into(),
                currency: usd(),
            })
            .await
            .unwrap();
        svc.create_user(
            lid1,
            NewUserName {
                display_name: "Alice".into(),
            },
        )
        .await
        .unwrap();
        svc.create_user(
            lid2,
            NewUserName {
                display_name: "Bob".into(),
            },
        )
        .await
        .unwrap();
        let all = svc.list_all_users().await.unwrap();
        assert_eq!(all.len(), 2);
    }

    #[tokio::test]
    async fn test_list_all_users_deduplicates_by_user_id() {
        let svc = open().await;
        let lid1 = svc
            .create_ledger(NewLedger {
                name: "L1".into(),
                currency: usd(),
            })
            .await
            .unwrap();
        let lid2 = svc
            .create_ledger(NewLedger {
                name: "L2".into(),
                currency: usd(),
            })
            .await
            .unwrap();
        let alice = svc
            .create_user(
                lid1,
                NewUserName {
                    display_name: "Alice".into(),
                },
            )
            .await
            .unwrap();
        svc.add_user(
            lid2,
            NewUser {
                user_id: alice.user_id,
                display_name: "Alice".into(),
            },
        )
        .await
        .unwrap();
        let all = svc.list_all_users().await.unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].user_id, alice.user_id);
    }

    // unused import suppression
    #[allow(dead_code)]
    fn _use_bill_id(_: BillId) {}
}
