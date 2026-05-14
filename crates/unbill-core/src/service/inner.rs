// UnbillService: top-level facade consumed by CLI and Tauri.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::broadcast;

use crate::conflict::{self, ConflictGroup};
use crate::error::{Result, UnbillError};
use garde::validate::Validate as _;

use crate::model::{
    BillId, Currency, Device, EffectiveBills, LedgerId, LedgerMeta, NewBill, NewDevice, NewLedger,
    NewUser, NewUserName, NodeId, Timestamp, User, UserId,
};
#[cfg(feature = "local")]
use crate::model::{Invitation, InviteToken};
use crate::settlement;
use unbill_event::ServiceEvent;
use unbill_storage::LedgerDoc;
use unbill_storage::LockableStore;
use unbill_storage::{load_device_labels, save_device_labels};
#[cfg(feature = "local")]
use unbill_storage::{load_pending_invitations, save_pending_invitations};

pub struct UnbillService<S>
where
    S: LockableStore + Send + Sync + 'static,
    for<'a> S::Guard<'a>: Send,
{
    pub(crate) store: Arc<S>,
    pub(crate) device_id: NodeId,
    pub(crate) events: broadcast::Sender<ServiceEvent>,
    #[cfg(feature = "remote")]
    pub(crate) server_client: Option<Arc<unbill_server_client::ServerClient>>,
    #[cfg(feature = "local")]
    pub(crate) endpoint: std::sync::Mutex<Option<Arc<crate::net::UnbillEndpoint>>>,
}

impl<S> UnbillService<S>
where
    S: LockableStore + Send + Sync + 'static,
    for<'a> S::Guard<'a>: Send,
{
    /// Open the service: initialize device identity via the store, then open.
    ///
    /// All store-backed data (ledgers, pending invitations, pending user
    /// tokens, local users) is loaded on demand and never cached in memory.
    pub async fn open(store: Arc<S>) -> Result<Arc<Self>> {
        store.lock().await?.create_secret_key().await?;
        let device_id = store.lock().await?.get_device_id().await?;
        let (events, _) = broadcast::channel(256);
        Ok(Arc::new(Self {
            store,
            device_id,
            events,
            #[cfg(feature = "remote")]
            server_client: None,
            #[cfg(feature = "local")]
            endpoint: std::sync::Mutex::new(None),
        }))
    }

    /// Open the service connected to a remote `unbill-server`.
    ///
    /// `base_url` is the server root (e.g. `https://example.com`).
    /// `api_key` is the Bearer token.
    /// Network operations (sync, invite, join) are delegated to the server
    /// via `unbill-server-client`.
    #[cfg(feature = "remote")]
    pub async fn open_remote(
        store: Arc<S>,
        base_url: String,
        api_key: String,
    ) -> Result<Arc<Self>> {
        let device_id = store.lock().await?.get_device_id().await?;
        let (events, _) = broadcast::channel(256);
        Ok(Arc::new(Self {
            store,
            device_id,
            events,
            server_client: Some(Arc::new(unbill_server_client::ServerClient::new(
                base_url, api_key,
            ))),
            #[cfg(feature = "local")]
            endpoint: std::sync::Mutex::new(None),
        }))
    }

    // -----------------------------------------------------------------------
    // Ledger lifecycle
    // -----------------------------------------------------------------------

    pub async fn create_ledger(&self, input: NewLedger) -> Result<LedgerId> {
        input
            .validate()
            .map_err(|e| UnbillError::Validation(e.to_string()))?;
        let ledger_id = LedgerId::new();
        let now = Timestamp::now();

        let mut doc = LedgerDoc::new(ledger_id, input.name.clone(), input.currency, now)?;
        doc.add_device(
            NewDevice {
                node_id: self.device_id.clone(),
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
        self.store.lock().await?.save_ledger_meta(&meta).await?;
        self.store
            .lock()
            .await?
            .save_ledger(&ledger_id.to_string(), &mut doc)
            .await?;

        Ok(ledger_id)
    }

    pub async fn list_ledgers(&self) -> Result<Vec<LedgerMeta>> {
        Ok(self.store.lock().await?.list_ledgers().await?)
    }

    pub async fn delete_ledger(&self, ledger_id: LedgerId) -> Result<()> {
        self.store
            .lock()
            .await?
            .delete_ledger(&ledger_id.to_string())
            .await?;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Bills
    // -----------------------------------------------------------------------

    pub async fn add_bill(&self, ledger_id: LedgerId, input: NewBill) -> Result<BillId> {
        input
            .validate()
            .map_err(|e| UnbillError::Validation(e.to_string()))?;
        let mut doc = self.load_doc(ledger_id).await?;
        let bill_id = doc.add_bill(input, self.device_id.clone(), Timestamp::now())?;
        self.store
            .lock()
            .await?
            .save_ledger(&ledger_id.to_string(), &mut doc)
            .await?;
        self.touch_meta(ledger_id).await?;
        let _ = self.events.send(ServiceEvent::LedgerUpdated {
            ledger_id: ledger_id.to_string(),
        });
        Ok(bill_id)
    }

    pub async fn list_bills(&self, ledger_id: LedgerId) -> Result<EffectiveBills> {
        self.load_doc(ledger_id).await?.list_bills()
    }

    // -----------------------------------------------------------------------
    // Users
    // -----------------------------------------------------------------------

    pub async fn add_user(&self, ledger_id: LedgerId, input: NewUser) -> Result<()> {
        input
            .validate()
            .map_err(|e| UnbillError::Validation(e.to_string()))?;
        let mut doc = self.load_doc(ledger_id).await?;
        doc.add_user(input, Timestamp::now())?;
        self.store
            .lock()
            .await?
            .save_ledger(&ledger_id.to_string(), &mut doc)
            .await?;
        self.touch_meta(ledger_id).await?;
        let _ = self.events.send(ServiceEvent::LedgerUpdated {
            ledger_id: ledger_id.to_string(),
        });
        Ok(())
    }

    pub async fn list_users(&self, ledger_id: LedgerId) -> Result<Vec<User>> {
        self.load_doc(ledger_id).await?.list_users()
    }

    /// Create a brand-new user, add them to the ledger, and return the created `User`.
    pub async fn create_user(&self, ledger_id: LedgerId, input: NewUserName) -> Result<User> {
        input
            .validate()
            .map_err(|e| UnbillError::Validation(e.to_string()))?;
        let user_id = UserId::new();
        let now = Timestamp::now();
        let mut doc = self.load_doc(ledger_id).await?;
        doc.add_user(
            NewUser {
                user_id,
                display_name: input.display_name.clone(),
            },
            now,
        )?;
        self.store
            .lock()
            .await?
            .save_ledger(&ledger_id.to_string(), &mut doc)
            .await?;
        self.touch_meta(ledger_id).await?;
        let _ = self.events.send(ServiceEvent::LedgerUpdated {
            ledger_id: ledger_id.to_string(),
        });
        Ok(User {
            user_id,
            display_name: input.display_name,
            added_at: now,
        })
    }

    /// Return all unique users across every ledger on this device, deduplicated by `user_id`.
    ///
    /// Use this to populate the user picker when adding a member to a ledger.
    pub async fn list_all_users(&self) -> Result<Vec<User>> {
        let mut seen = std::collections::HashSet::new();
        let mut result = Vec::new();
        let metas = self.store.lock().await?.list_ledgers().await?;
        for meta in metas {
            let doc = self.load_doc(meta.ledger_id).await?;
            for user in doc.list_users()? {
                if seen.insert(user.user_id) {
                    result.push(user);
                }
            }
        }
        Ok(result)
    }

    // -----------------------------------------------------------------------
    // Devices
    // -----------------------------------------------------------------------

    pub async fn add_device(&self, ledger_id: LedgerId, input: NewDevice) -> Result<()> {
        let mut doc = self.load_doc(ledger_id).await?;
        doc.add_device(input, Timestamp::now())?;
        self.store
            .lock()
            .await?
            .save_ledger(&ledger_id.to_string(), &mut doc)
            .await?;
        self.touch_meta(ledger_id).await?;
        Ok(())
    }

    pub async fn list_devices(&self, ledger_id: LedgerId) -> Result<Vec<Device>> {
        self.load_doc(ledger_id).await?.list_devices()
    }

    pub async fn list_device_labels(&self) -> Result<HashMap<String, String>> {
        let guard = self.store.lock().await?;
        load_device_labels(&*guard).await
    }

    pub async fn set_device_label(&self, node_id: NodeId, label: String) -> Result<()> {
        let guard = self.store.lock().await?;
        let mut labels = load_device_labels(&*guard).await?;
        let key = node_id.to_string();
        let trimmed = label.trim();
        if trimmed.is_empty() {
            labels.remove(&key);
        } else {
            labels.insert(key, trimmed.to_owned());
        }
        save_device_labels(&*guard, &labels).await
    }

    // -----------------------------------------------------------------------
    // Settlement
    // -----------------------------------------------------------------------

    /// Compute net settlement for a user across all ledgers they participate in,
    /// grouped by currency. Returns one `Settlement` per currency.
    pub async fn compute_settlement_for_user(
        &self,
        user_id: UserId,
    ) -> Result<Vec<settlement::Settlement>> {
        let mut by_currency: std::collections::HashMap<
            Currency,
            std::collections::HashMap<UserId, i64>,
        > = std::collections::HashMap::new();

        let metas = self.store.lock().await?.list_ledgers().await?;
        for meta in metas {
            let doc = self.load_doc(meta.ledger_id).await?;
            let users = doc.list_users()?;
            // Only aggregate ledgers where this user is active.
            if users.iter().any(|user| user.user_id == user_id) {
                let currency = doc.get_ledger()?.currency;
                let bills = doc.list_bills()?;
                let balances = by_currency.entry(currency).or_default();
                settlement::accumulate_balances(&users, &bills, balances);
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
        let doc = self.load_doc(ledger_id).await?;
        let ledger = doc.get_ledger()?;
        Ok(settlement::compute_settlement(&ledger))
    }

    // -----------------------------------------------------------------------
    // Conflict detection
    // -----------------------------------------------------------------------

    pub async fn detect_conflicts(&self, ledger_id: LedgerId) -> Result<Vec<ConflictGroup>> {
        let doc = self.load_doc(ledger_id).await?;
        let all_bills = doc.list_all_bills()?;
        Ok(conflict::detect(&all_bills))
    }

    // -----------------------------------------------------------------------
    // Invitations and sync
    // -----------------------------------------------------------------------

    /// Generate a join invite URL for `ledger_id` and store the pending invitation.
    ///
    /// URL format: `unbill://join/<ledger_id>/<host_node_id>/<token_hex>`
    pub async fn create_invitation(&self, ledger_id: LedgerId) -> Result<String> {
        #[cfg(feature = "remote")]
        if let Some(client) = self.server_client.as_deref() {
            return client
                .create_invitation(&ledger_id.to_string())
                .await
                .map_err(|e| UnbillError::Network(e.to_string()));
        }

        #[cfg(feature = "local")]
        {
            // Check the ledger exists locally.
            let _ = self.load_doc(ledger_id).await?;
            let token = InviteToken::generate();
            let now = Timestamp::now();
            let invitation = Invitation {
                token: token.clone(),
                ledger_id,
                created_by_device: self.device_id.clone(),
                created_at: now,
                expires_at: Timestamp::from_millis(now.as_millis() + 24 * 3600 * 1000),
            };
            {
                let guard = self.store.lock().await?;
                let mut map = load_pending_invitations(&*guard).await?;
                map.insert(token.to_string(), invitation);
                save_pending_invitations(&*guard, &map).await?;
            }
            return Ok(format!(
                "unbill://join/{}/{}/{}",
                ledger_id, self.device_id, token
            ));
        }

        #[allow(unreachable_code)]
        Err(UnbillError::NoNetworkFeature)
    }

    /// Accept a join invite URL and join the ledger hosted by the inviting device.
    ///
    /// URL format: `unbill://join/<ledger_id>/<host_node_id>/<token_hex>`
    /// `label` is an optional device-local nickname for the host device.
    pub async fn join_ledger(self: &Arc<Self>, url: &str, label: String) -> Result<()> {
        #[cfg(feature = "remote")]
        if let Some(client) = self.server_client.as_deref() {
            let local_label = (!label.trim().is_empty()).then_some(label.trim());
            return client
                .join_ledger(url, local_label)
                .await
                .map_err(|e| UnbillError::Network(e.to_string()));
        }

        #[cfg(feature = "local")]
        {
            use crate::net::{JoinRequest, UnbillEndpoint};
            let (ledger_id, host, token) = parse_join_url(url)?;
            let local_label = (!label.trim().is_empty()).then_some(label.trim().to_owned());
            let request = JoinRequest { token, ledger_id };
            let ep = self.endpoint.lock().unwrap().clone();
            if let Some(ep) = ep {
                return ep
                    .join_ledger_inner(host, local_label, request, &self.store, &self.events)
                    .await;
            }
            let key = self.store.lock().await?.get_secret_key().await?;
            let ep = UnbillEndpoint::bind(&key).await?;
            let result = ep
                .join_ledger_inner(host, local_label, request, &self.store, &self.events)
                .await;
            ep.close().await;
            return result;
        }

        #[allow(unreachable_code)]
        Err(UnbillError::NoNetworkFeature)
    }

    /// Dial `peer` and run the full sync exchange for all shared ledgers.
    pub async fn sync_once(self: &Arc<Self>, peer: NodeId) -> Result<()> {
        #[cfg(feature = "remote")]
        if let Some(client) = self.server_client.as_deref() {
            return client
                .sync_with_peer(peer.as_str())
                .await
                .map_err(|e| UnbillError::Network(e.to_string()));
        }

        #[cfg(feature = "local")]
        {
            use crate::net::UnbillEndpoint;
            let ep = self.endpoint.lock().unwrap().clone();
            if let Some(ep) = ep {
                return ep.sync_once_inner(peer, &self.store, &self.events).await;
            }
            let key = self.store.lock().await?.get_secret_key().await?;
            let ep = UnbillEndpoint::bind(&key).await?;
            let result = ep.sync_once_inner(peer, &self.store, &self.events).await;
            ep.close().await;
            return result;
        }

        #[allow(unreachable_code)]
        Err(UnbillError::NoNetworkFeature)
    }

    /// Open an endpoint and accept incoming sync/join/user-transfer connections until
    /// an error occurs or the process is interrupted.
    ///
    /// Prints the local `NodeId` to stdout so peers know what to dial.
    #[cfg(feature = "local")]
    pub async fn accept_loop(self: &Arc<Self>) -> Result<()> {
        use crate::net::UnbillEndpoint;
        let key = self.store.lock().await?.get_secret_key().await?;
        let ep = Arc::new(UnbillEndpoint::bind(&key).await?);
        ep.wait_for_ready().await;
        println!("listening on: {}", ep.node_id());
        *self.endpoint.lock().unwrap() = Some(Arc::clone(&ep));
        let result = ep
            .accept_loop_inner(Arc::clone(&self.store), self.events.clone())
            .await;
        *self.endpoint.lock().unwrap() = None;
        ep.close().await;
        result
    }

    // -----------------------------------------------------------------------
    // Events
    // -----------------------------------------------------------------------

    pub fn device_id(&self) -> NodeId {
        self.device_id.clone()
    }

    pub fn store(&self) -> &Arc<S> {
        &self.store
    }

    pub fn subscribe(&self) -> broadcast::Receiver<ServiceEvent> {
        self.events.subscribe()
    }

    // -----------------------------------------------------------------------
    // Internals
    // -----------------------------------------------------------------------

    async fn load_doc(&self, ledger_id: LedgerId) -> Result<LedgerDoc> {
        self.store
            .lock()
            .await?
            .load_ledger(&ledger_id.to_string())
            .await?
            .ok_or_else(|| UnbillError::LedgerNotFound(ledger_id.to_string()))
    }

    /// Update `updated_at` in the stored metadata for a ledger.
    async fn touch_meta(&self, ledger_id: LedgerId) -> Result<()> {
        let mut metas = self.store.lock().await?.list_ledgers().await?;
        if let Some(meta) = metas.iter_mut().find(|m| m.ledger_id == ledger_id) {
            meta.updated_at = Timestamp::now();
            self.store.lock().await?.save_ledger_meta(meta).await?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Parse `unbill://join/<ledger_id>/<host_node_id>/<token_hex>`.
#[cfg(feature = "local")]
fn parse_join_url(url: &str) -> Result<(String, NodeId, String)> {
    let path = url
        .strip_prefix("unbill://join/")
        .ok_or_else(|| UnbillError::InvalidUrl(format!("invalid join URL: {url:?}")))?;
    let parts: Vec<&str> = path.splitn(3, '/').collect();
    if parts.len() != 3 {
        return Err(UnbillError::InvalidUrl(format!(
            "invalid join URL (expected ledger_id/host_node_id/token): {url:?}"
        )));
    }
    let ledger_id = parts[0].to_string();
    let host = parts[1]
        .parse::<NodeId>()
        .map_err(|e| UnbillError::InvalidUrl(format!("invalid host node ID in URL: {e}")))?;
    let token = parts[2].to_string();
    Ok((ledger_id, host, token))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{BillId, NewLedger, Share};
    use unbill_store_memory::LockedInMemoryStore;

    fn mem_store() -> Arc<LockedInMemoryStore> {
        Arc::new(LockedInMemoryStore::default())
    }

    async fn open() -> Arc<UnbillService<LockedInMemoryStore>> {
        UnbillService::open(mem_store()).await.unwrap()
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

    async fn seed_users(svc: &UnbillService<LockedInMemoryStore>, ledger_id: LedgerId) {
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

    // --- create / list / delete ledger ---

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
    async fn test_delete_ledger_removes_it() {
        let svc = open().await;
        let id = svc
            .create_ledger(NewLedger {
                name: "Trip".into(),
                currency: usd(),
            })
            .await
            .unwrap();
        svc.delete_ledger(id).await.unwrap();
        assert!(svc.list_ledgers().await.unwrap().is_empty());
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

        // Ledger 1: Alice pays $60, Alice+Bob split → Bob owes Alice $30.
        let lid1 = svc
            .create_ledger(NewLedger {
                name: "L1".into(),
                currency: usd(),
            })
            .await
            .unwrap();
        seed_users(&svc, lid1).await;
        svc.add_bill(lid1, two_way_bill(6000)).await.unwrap();

        // Ledger 2: Bob pays $20, Alice+Bob split → Alice owes Bob $10.
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

        // Net: Bob owes Alice $30 - $10 = $20. Both ledgers are USD → one group.
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

        // USD ledger: Alice pays $60, Alice+Bob split → Bob owes Alice $30.
        let lid_usd = svc
            .create_ledger(NewLedger {
                name: "USD ledger".into(),
                currency: usd(),
            })
            .await
            .unwrap();
        seed_users(&svc, lid_usd).await;
        svc.add_bill(lid_usd, two_way_bill(6000)).await.unwrap();

        // EUR ledger: Alice pays €40, Alice+Bob split → Bob owes Alice €20.
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

        // Must produce two separate groups — one USD, one EUR — not a single merged total.
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
            let svc = UnbillService::open(Arc::clone(&store)).await.unwrap();
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
        // Re-open with the same store (simulates a restart).
        let svc2 = UnbillService::open(Arc::clone(&store)).await.unwrap();
        let bills = svc2.list_bills(lid).await.unwrap();
        assert_eq!(bills.0.len(), 1);
        assert_eq!(bills.0[0].amount_cents, 120000);
    }

    #[tokio::test]
    async fn test_device_key_stable_across_restarts() {
        let store = mem_store();
        let svc1 = UnbillService::open(Arc::clone(&store)).await.unwrap();
        let svc2 = UnbillService::open(Arc::clone(&store)).await.unwrap();
        assert_eq!(svc1.device_id, svc2.device_id);
    }

    #[tokio::test]
    async fn test_device_labels_survive_restart() {
        let store = mem_store();
        let peer = NodeId::from_seed(9);
        {
            let svc = UnbillService::open(Arc::clone(&store)).await.unwrap();
            svc.set_device_label(peer.clone(), "Kitchen iPad".into())
                .await
                .unwrap();
        }
        let svc2 = UnbillService::open(Arc::clone(&store)).await.unwrap();
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
        // Create Alice in L1, then add the same user to L2.
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
