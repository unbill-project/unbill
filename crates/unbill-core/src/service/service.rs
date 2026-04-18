// UnbillService: top-level facade consumed by CLI and Tauri.

use std::collections::HashMap;
use std::sync::Arc;

use dashmap::DashMap;
use rand::RngCore as _;
use tokio::sync::{broadcast, Mutex};

use crate::doc::LedgerDoc;
use crate::error::{Result, UnbillError};
use crate::model::{
    Currency, Device, EffectiveBill, Invitation, InviteToken, LedgerMeta, Member, NewBill,
    NewDevice, NewMember, NodeId, Timestamp, Ulid,
};
use crate::net::{PendingIdentityTokens, PendingInvitations};
use crate::settlement;
use crate::storage::LedgerStore;

pub struct UnbillService {
    pub(crate) store: Arc<dyn LedgerStore>,
    pub(crate) device_id: NodeId,
    pub(crate) secret_key: iroh::SecretKey,
    /// Eagerly loaded in-memory ledger documents, keyed by ledger ID string.
    pub(crate) ledgers: DashMap<String, Arc<Mutex<LedgerDoc>>>,
    /// Pending join invitations (token hex → Invitation). In-memory only.
    pub(crate) pending_invitations: PendingInvitations,
    /// Pending identity-transfer tokens (token hex → (user_id, display_name)). In-memory only.
    pub(crate) pending_identity_tokens: PendingIdentityTokens,
    pub(crate) events: broadcast::Sender<ServiceEvent>,
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
        let (device_id, secret_key) = load_or_create_device_key(&*store).await?;

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
            secret_key,
            ledgers,
            pending_invitations: Arc::new(std::sync::Mutex::new(HashMap::new())),
            pending_identity_tokens: Arc::new(std::sync::Mutex::new(HashMap::new())),
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
        input: NewBill,
    ) -> Result<()> {
        let bill_ulid = parse_ulid(bill_id)?;
        let doc_mutex = self.get_doc(ledger_id)?;
        let mut doc = doc_mutex.lock().await;
        doc.amend_bill(&bill_ulid, input, self.device_id, Timestamp::now())?;
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

    pub async fn list_members(&self, ledger_id: &str) -> Result<Vec<Member>> {
        let doc_mutex = self.get_doc(ledger_id)?;
        let doc = doc_mutex.lock().await;
        doc.list_members()
    }

    // -----------------------------------------------------------------------
    // Devices
    // -----------------------------------------------------------------------

    pub async fn add_device(&self, ledger_id: &str, input: NewDevice) -> Result<()> {
        let doc_mutex = self.get_doc(ledger_id)?;
        let mut doc = doc_mutex.lock().await;
        doc.add_device(input, Timestamp::now())?;
        let bytes = doc.save();
        drop(doc);

        self.store.save_ledger_bytes(ledger_id, &bytes).await?;
        self.touch_meta(ledger_id).await?;
        Ok(())
    }

    pub async fn list_devices(&self, ledger_id: &str) -> Result<Vec<Device>> {
        let doc_mutex = self.get_doc(ledger_id)?;
        let doc = doc_mutex.lock().await;
        doc.list_devices()
    }

    // -----------------------------------------------------------------------
    // Settlement
    // -----------------------------------------------------------------------

    /// Compute net settlement for a user across all ledgers they participate in.
    ///
    /// Balances are accumulated from every ledger where the user appears as a
    /// member or in a bill, then minimum-cash-flow is applied to the combined
    /// map. The result is filtered to transactions that involve the given user.
    pub async fn compute_settlement_for_user(
        &self,
        user_id: &str,
    ) -> Result<settlement::Settlement> {
        let user_ulid = parse_ulid(user_id)?;
        let mut balances: std::collections::HashMap<crate::model::Ulid, i64> =
            std::collections::HashMap::new();

        for entry in self.ledgers.iter() {
            let doc = entry.value().lock().await;
            let members = doc.list_members()?;
            // Only aggregate ledgers where this user is an active member.
            if members.iter().any(|m| m.user_id == user_ulid) {
                let bills = doc.list_bills()?;
                settlement::accumulate_balances(&members, &bills, &mut balances);
            }
        }

        let full = settlement::compute_from_balances(balances);
        // Keep only transactions involving this user.
        let transactions = full
            .transactions
            .into_iter()
            .filter(|t| t.from_user_id == user_ulid || t.to_user_id == user_ulid)
            .collect();
        Ok(settlement::Settlement { transactions })
    }

    // -----------------------------------------------------------------------
    // Invitations and sync
    // -----------------------------------------------------------------------

    /// Generate a join invite URL for `ledger_id` and store the pending invitation.
    ///
    /// URL format: `unbill://join/<ledger_id>/<host_node_id>/<token_hex>`
    pub async fn create_invitation(&self, ledger_id: &str) -> Result<String> {
        let ledger_ulid = parse_ulid(ledger_id)?;
        // Check the ledger exists locally.
        let _ = self.get_doc(ledger_id)?;
        let token = InviteToken::generate();
        let now = Timestamp::now();
        let invitation = Invitation {
            token: token.clone(),
            ledger_id: ledger_ulid,
            created_by_device: self.device_id,
            created_at: now,
            expires_at: Timestamp::from_millis(now.as_millis() + 24 * 3600 * 1000),
        };
        self.pending_invitations
            .lock()
            .unwrap()
            .insert(token.to_string(), invitation);
        Ok(format!(
            "unbill://join/{}/{}/{}",
            ledger_id, self.device_id, token
        ))
    }

    /// Generate an identity-share URL for `user_id` and store the pending token.
    ///
    /// URL format: `unbill://identity/<host_node_id>/<token_hex>`
    pub async fn create_identity_share(&self, user_id: &str) -> Result<String> {
        let user_ulid = parse_ulid(user_id)?;
        let identities = load_identities(&*self.store).await?;
        let identity = identities
            .iter()
            .find(|i| i.user_id == user_ulid)
            .ok_or_else(|| UnbillError::Other(anyhow::anyhow!("identity not found: {user_id}")))?;
        let display_name = identity.display_name.clone();
        let mut token_bytes = [0u8; 32];
        rand::rngs::OsRng.fill_bytes(&mut token_bytes);
        let token_hex: String = token_bytes.iter().map(|b| format!("{b:02x}")).collect();
        self.pending_identity_tokens
            .lock()
            .unwrap()
            .insert(token_hex.clone(), (user_ulid, display_name));
        Ok(format!(
            "unbill://identity/{}/{}",
            self.device_id, token_hex
        ))
    }

    /// Accept a join invite URL and join the ledger hosted by the inviting device.
    ///
    /// URL format: `unbill://join/<ledger_id>/<host_node_id>/<token_hex>`
    /// `label` is a human-readable name for this device recorded in the ledger.
    pub async fn join_ledger(self: &Arc<Self>, url: &str, label: String) -> Result<()> {
        use crate::net::{JoinRequest, UnbillEndpoint};
        let (ledger_id, host, token) = parse_join_url(url)?;
        let request = JoinRequest { token, ledger_id, label };
        let ep = UnbillEndpoint::bind(self.secret_key.clone())
            .await
            .map_err(UnbillError::Other)?;
        let result = ep.join_ledger_inner(host, request, self).await;
        ep.close().await;
        result.map_err(UnbillError::Other)
    }

    /// Fetch an identity from another device using an `unbill://identity/...` URL.
    ///
    /// URL format: `unbill://identity/<host_node_id>/<token_hex>`
    pub async fn fetch_identity(self: &Arc<Self>, url: &str) -> Result<()> {
        use crate::net::UnbillEndpoint;
        let (host, token) = parse_identity_url(url)?;
        let ep = UnbillEndpoint::bind(self.secret_key.clone())
            .await
            .map_err(UnbillError::Other)?;
        let result = ep.import_identity_inner(host, token, self).await;
        ep.close().await;
        result.map_err(UnbillError::Other)
    }

    /// Dial `peer` and run the full sync exchange for all shared ledgers.
    pub async fn sync_once(self: &Arc<Self>, peer: NodeId) -> Result<()> {
        use crate::net::UnbillEndpoint;
        let ep = UnbillEndpoint::bind(self.secret_key.clone())
            .await
            .map_err(UnbillError::Other)?;
        let result = ep.sync_once_inner(peer, self).await;
        ep.close().await;
        result.map_err(UnbillError::Other)
    }

    /// Open an endpoint and accept incoming sync/join/identity connections until
    /// an error occurs or the process is interrupted.
    ///
    /// Prints the local `NodeId` to stdout so peers know what to dial.
    pub async fn accept_loop(self: &Arc<Self>) -> Result<()> {
        use crate::net::UnbillEndpoint;
        let ep = UnbillEndpoint::bind(self.secret_key.clone())
            .await
            .map_err(UnbillError::Other)?;
        println!("listening on: {}", ep.node_id());
        let result = ep.accept_loop_inner(Arc::clone(self)).await;
        ep.close().await;
        result.map_err(UnbillError::Other)
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
    // Identity
    // -----------------------------------------------------------------------

    /// Return all user identities stored on this device.
    pub async fn list_identities(&self) -> Result<Vec<Identity>> {
        load_identities(&*self.store).await
    }

    /// Add a new identity (fresh user ID + display name) to this device.
    pub async fn add_identity(&self, display_name: String) -> Result<Identity> {
        let identity = Identity {
            user_id: Ulid::new(),
            display_name,
        };
        let mut identities = load_identities(&*self.store).await?;
        identities.push(identity.clone());
        save_identities(&*self.store, &identities).await?;
        Ok(identity)
    }

    /// Import an existing identity onto this device.
    pub async fn import_identity(&self, user_id: Ulid, display_name: String) -> Result<Identity> {
        let identity = Identity {
            user_id,
            display_name,
        };
        let mut identities = load_identities(&*self.store).await?;
        if !identities.iter().any(|i| i.user_id == identity.user_id) {
            identities.push(identity.clone());
            save_identities(&*self.store, &identities).await?;
        }
        Ok(identity)
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
// Identity type
// ---------------------------------------------------------------------------

/// A user identity stored on this device.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Identity {
    pub user_id: Ulid,
    pub display_name: String,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const IDENTITIES_KEY: &str = "identities.json";

async fn load_identities(store: &dyn LedgerStore) -> Result<Vec<Identity>> {
    match store.load_device_meta(IDENTITIES_KEY).await? {
        None => Ok(vec![]),
        Some(bytes) => serde_json::from_slice(&bytes)
            .map_err(|e| UnbillError::Other(anyhow::anyhow!("identities.json: {e}"))),
    }
}

async fn save_identities(store: &dyn LedgerStore, identities: &[Identity]) -> Result<()> {
    let bytes = serde_json::to_vec(identities)
        .map_err(|e| UnbillError::Other(anyhow::anyhow!("serialize identities: {e}")))?;
    store.save_device_meta(IDENTITIES_KEY, &bytes).await?;
    Ok(())
}

async fn load_or_create_device_key(store: &dyn LedgerStore) -> Result<(NodeId, iroh::SecretKey)> {
    if let Some(bytes) = store.load_device_meta("device_key.bin").await? {
        let arr: [u8; 32] = bytes
            .try_into()
            .map_err(|_| UnbillError::Other(anyhow::anyhow!("device_key.bin: wrong length")))?;
        let secret = iroh::SecretKey::from(arr);
        Ok((NodeId::from_node_id(secret.public()), secret))
    } else {
        let mut arr = [0u8; 32];
        rand::rngs::OsRng.fill_bytes(&mut arr);
        let secret = iroh::SecretKey::from(arr);
        store.save_device_meta("device_key.bin", &arr).await?;
        Ok((NodeId::from_node_id(secret.public()), secret))
    }
}

fn parse_ulid(s: &str) -> Result<Ulid> {
    Ulid::from_string(s).map_err(|e| UnbillError::Other(anyhow::anyhow!("invalid ULID {s:?}: {e}")))
}

/// Parse `unbill://join/<ledger_id>/<host_node_id>/<token_hex>`.
fn parse_join_url(url: &str) -> Result<(String, NodeId, String)> {
    let path = url
        .strip_prefix("unbill://join/")
        .ok_or_else(|| UnbillError::Other(anyhow::anyhow!("invalid join URL: {url:?}")))?;
    let parts: Vec<&str> = path.splitn(3, '/').collect();
    if parts.len() != 3 {
        return Err(UnbillError::Other(anyhow::anyhow!(
            "invalid join URL (expected ledger_id/host_node_id/token): {url:?}"
        )));
    }
    let ledger_id = parts[0].to_string();
    let host = parts[1]
        .parse::<NodeId>()
        .map_err(|e| UnbillError::Other(anyhow::anyhow!("invalid host node ID in URL: {e}")))?;
    let token = parts[2].to_string();
    Ok((ledger_id, host, token))
}

/// Parse `unbill://identity/<host_node_id>/<token_hex>`.
fn parse_identity_url(url: &str) -> Result<(NodeId, String)> {
    let path = url
        .strip_prefix("unbill://identity/")
        .ok_or_else(|| UnbillError::Other(anyhow::anyhow!("invalid identity URL: {url:?}")))?;
    let parts: Vec<&str> = path.splitn(2, '/').collect();
    if parts.len() != 2 {
        return Err(UnbillError::Other(anyhow::anyhow!(
            "invalid identity URL (expected host_node_id/token): {url:?}"
        )));
    }
    let host = parts[0]
        .parse::<NodeId>()
        .map_err(|e| UnbillError::Other(anyhow::anyhow!("invalid host node ID in URL: {e}")))?;
    let token = parts[1].to_string();
    Ok((host, token))
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
                Share {
                    user_id: Ulid::from_u128(1),
                    shares: 1,
                },
                Share {
                    user_id: Ulid::from_u128(2),
                    shares: 1,
                },
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
            NewBill {
                payer_user_id: Ulid::from_u128(1),
                amount_cents: 4000,
                description: "Lunch".into(),
                shares: vec![
                    Share { user_id: Ulid::from_u128(1), shares: 1 },
                    Share { user_id: Ulid::from_u128(2), shares: 1 },
                ],
            },
        )
        .await
        .unwrap();

        let bills = svc.list_bills(&lid).await.unwrap();
        assert_eq!(bills[0].amount_cents, 4000);
        assert!(bills[0].was_amended);
    }

    // --- settlement ---

    #[tokio::test]
    async fn test_compute_settlement_no_bills_is_empty() {
        let svc = open().await;
        let lid = svc
            .create_ledger("Empty".into(), usd().into())
            .await
            .unwrap();
        seed_members(&svc, &lid).await;
        let alice = Ulid::from_u128(1).to_string();
        let s = svc.compute_settlement_for_user(&alice).await.unwrap();
        assert!(s.transactions.is_empty());
    }

    #[tokio::test]
    async fn test_compute_settlement_cross_ledger() {
        let svc = open().await;

        // Ledger 1: Alice pays $60, Alice+Bob split → Bob owes Alice $30.
        let lid1 = svc.create_ledger("L1".into(), usd().into()).await.unwrap();
        seed_members(&svc, &lid1).await;
        let (_, bill1) = two_way_bill("Rent", 6000, &lid1);
        svc.add_bill(&lid1, bill1).await.unwrap();

        // Ledger 2: Bob pays $20, Alice+Bob split → Alice owes Bob $10.
        let lid2 = svc.create_ledger("L2".into(), usd().into()).await.unwrap();
        seed_members(&svc, &lid2).await;
        let bob_pays = NewBill {
            payer_user_id: Ulid::from_u128(2),
            amount_cents: 2000,
            description: "Utilities".into(),
            shares: vec![
                Share {
                    user_id: Ulid::from_u128(1),
                    shares: 1,
                },
                Share {
                    user_id: Ulid::from_u128(2),
                    shares: 1,
                },
            ],
        };
        svc.add_bill(&lid2, bob_pays).await.unwrap();

        // Net: Bob owes Alice $30 - $10 = $20.
        let alice = Ulid::from_u128(1).to_string();
        let s = svc.compute_settlement_for_user(&alice).await.unwrap();
        assert_eq!(s.transactions.len(), 1);
        assert_eq!(s.transactions[0].amount_cents, 2000);
        assert_eq!(s.transactions[0].from_user_id, Ulid::from_u128(2));
        assert_eq!(s.transactions[0].to_user_id, Ulid::from_u128(1));
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

    // --- identities ---

    #[tokio::test]
    async fn test_add_identity_appears_in_list() {
        let svc = open().await;
        let identity = svc.add_identity("Alice".into()).await.unwrap();
        let list = svc.list_identities().await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].user_id, identity.user_id);
        assert_eq!(list[0].display_name, "Alice");
    }

    #[tokio::test]
    async fn test_multiple_identities_stored() {
        let svc = open().await;
        svc.add_identity("Alice".into()).await.unwrap();
        svc.add_identity("Bob".into()).await.unwrap();
        let list = svc.list_identities().await.unwrap();
        assert_eq!(list.len(), 2);
    }

    #[tokio::test]
    async fn test_import_identity_deduplicates() {
        let svc = open().await;
        let id = svc.add_identity("Alice".into()).await.unwrap();
        svc.import_identity(id.user_id, "Alice".into())
            .await
            .unwrap();
        assert_eq!(svc.list_identities().await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_identities_survive_restart() {
        let store = mem_store();
        let user_id = {
            let svc = UnbillService::open(Arc::clone(&store)).await.unwrap();
            svc.add_identity("Alice".into()).await.unwrap().user_id
        };
        let svc2 = UnbillService::open(Arc::clone(&store)).await.unwrap();
        let list = svc2.list_identities().await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].user_id, user_id);
    }
}
