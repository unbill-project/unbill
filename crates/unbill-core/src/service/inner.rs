// UnbillService: top-level facade consumed by CLI and Tauri.

use std::collections::HashMap;
use std::sync::Arc;

use rand::TryRng as _;
use tokio::sync::broadcast;

use crate::doc::LedgerDoc;
use crate::error::{Result, UnbillError};
use crate::model::{
    Currency, Device, EffectiveBills, Invitation, InviteToken, LedgerMeta, NewBill, NewDevice,
    NewUser, NodeId, Timestamp, Ulid, User,
};
use crate::settlement;
use crate::storage::LedgerStore;

pub struct UnbillService {
    pub(crate) store: Arc<dyn LedgerStore>,
    pub(crate) device_id: NodeId,
    pub(crate) secret_key: iroh::SecretKey,
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
    /// Open the service: load or create the device key.
    ///
    /// All store-backed data (ledgers, pending invitations, pending user
    /// tokens, local users) is loaded on demand and never cached in memory.
    pub async fn open(store: Arc<dyn LedgerStore>) -> Result<Arc<Self>> {
        let (device_id, secret_key) = load_or_create_device_key(&*store).await?;
        let (events, _) = broadcast::channel(256);
        Ok(Arc::new(Self {
            store,
            device_id,
            secret_key,
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

        let mut doc = LedgerDoc::new(ledger_id, name.clone(), currency, now)?;
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

        Ok(id)
    }

    pub async fn list_ledgers(&self) -> Result<Vec<LedgerMeta>> {
        Ok(self.store.list_ledgers().await?)
    }

    pub async fn delete_ledger(&self, ledger_id: &str) -> Result<()> {
        self.store.delete_ledger(ledger_id).await?;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Bills
    // -----------------------------------------------------------------------

    pub async fn add_bill(&self, ledger_id: &str, input: NewBill) -> Result<String> {
        let mut doc = self.load_doc(ledger_id).await?;
        let bill_id = doc.add_bill(input, self.device_id, Timestamp::now())?;
        let bytes = doc.save();
        self.store.save_ledger_bytes(ledger_id, &bytes).await?;
        self.touch_meta(ledger_id).await?;
        Ok(bill_id.to_string())
    }

    pub async fn list_bills(&self, ledger_id: &str) -> Result<EffectiveBills> {
        self.load_doc(ledger_id).await?.list_bills()
    }

    // -----------------------------------------------------------------------
    // Users
    // -----------------------------------------------------------------------

    pub async fn add_user(&self, ledger_id: &str, input: NewUser) -> Result<()> {
        let mut doc = self.load_doc(ledger_id).await?;
        doc.add_user(input, Timestamp::now())?;
        let bytes = doc.save();
        self.store.save_ledger_bytes(ledger_id, &bytes).await?;
        self.touch_meta(ledger_id).await?;
        Ok(())
    }

    pub async fn list_users(&self, ledger_id: &str) -> Result<Vec<User>> {
        self.load_doc(ledger_id).await?.list_users()
    }

    // -----------------------------------------------------------------------
    // Devices
    // -----------------------------------------------------------------------

    pub async fn add_device(&self, ledger_id: &str, input: NewDevice) -> Result<()> {
        let mut doc = self.load_doc(ledger_id).await?;
        doc.add_device(input, Timestamp::now())?;
        let bytes = doc.save();
        self.store.save_ledger_bytes(ledger_id, &bytes).await?;
        self.touch_meta(ledger_id).await?;
        Ok(())
    }

    pub async fn list_devices(&self, ledger_id: &str) -> Result<Vec<Device>> {
        self.load_doc(ledger_id).await?.list_devices()
    }

    pub async fn list_device_labels(&self) -> Result<HashMap<String, String>> {
        load_device_labels(&*self.store).await
    }

    pub async fn set_device_label(&self, node_id: NodeId, label: String) -> Result<()> {
        let mut labels = load_device_labels(&*self.store).await?;
        let key = node_id.to_string();
        let trimmed = label.trim();
        if trimmed.is_empty() {
            labels.remove(&key);
        } else {
            labels.insert(key, trimmed.to_owned());
        }
        save_device_labels(&*self.store, &labels).await
    }

    // -----------------------------------------------------------------------
    // Settlement
    // -----------------------------------------------------------------------

    /// Compute net settlement for a user across all ledgers they participate in.
    ///
    /// Balances are accumulated from every ledger where the user appears as a
    /// user or in a bill, then minimum-cash-flow is applied to the combined
    /// map. The result is filtered to transactions that involve the given user.
    pub async fn compute_settlement_for_user(
        &self,
        user_id: &str,
    ) -> Result<settlement::Settlement> {
        let user_ulid = parse_ulid(user_id)?;
        let mut balances: std::collections::HashMap<crate::model::Ulid, i64> =
            std::collections::HashMap::new();

        for meta in self.store.list_ledgers().await? {
            let id = meta.ledger_id.to_string();
            let doc = self.load_doc(&id).await?;
            let users = doc.list_users()?;
            // Only aggregate ledgers where this user is active.
            if users.iter().any(|user| user.user_id == user_ulid) {
                let bills = doc.list_bills()?;
                settlement::accumulate_balances(&users, &bills, &mut balances);
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
        let _ = self.load_doc(ledger_id).await?;
        let token = InviteToken::generate();
        let now = Timestamp::now();
        let invitation = Invitation {
            token: token.clone(),
            ledger_id: ledger_ulid,
            created_by_device: self.device_id,
            created_at: now,
            expires_at: Timestamp::from_millis(now.as_millis() + 24 * 3600 * 1000),
        };
        {
            let mut map = load_pending_invitations(&*self.store).await?;
            map.insert(token.to_string(), invitation);
            save_pending_invitations(&*self.store, &map).await?;
        }
        Ok(format!(
            "unbill://join/{}/{}/{}",
            ledger_id, self.device_id, token
        ))
    }

    /// Generate a user-share URL for `user_id` and store the pending token.
    ///
    /// URL format: `unbill://user/<host_node_id>/<token_hex>`
    pub async fn create_local_user_share(&self, user_id: &str) -> Result<String> {
        let user_ulid = parse_ulid(user_id)?;
        let local_users = load_local_users(&*self.store).await?;
        let local_user = local_users
            .iter()
            .find(|i| i.user_id == user_ulid)
            .ok_or_else(|| {
                UnbillError::Other(anyhow::anyhow!("local user not found: {user_id}"))
            })?;
        let display_name = local_user.display_name.clone();
        let mut token_bytes = [0u8; 32];
        rand::rngs::SysRng
            .try_fill_bytes(&mut token_bytes)
            .expect("system RNG should generate user share tokens");
        let token_hex: String = token_bytes.iter().map(|b| format!("{b:02x}")).collect();
        {
            let mut map = load_pending_user_tokens(&*self.store).await?;
            map.insert(token_hex.clone(), (user_ulid, display_name));
            save_pending_user_tokens(&*self.store, &map).await?;
        }
        Ok(format!("unbill://user/{}/{}", self.device_id, token_hex))
    }

    /// Accept a join invite URL and join the ledger hosted by the inviting device.
    ///
    /// URL format: `unbill://join/<ledger_id>/<host_node_id>/<token_hex>`
    /// `label` is an optional device-local nickname for the host device.
    pub async fn join_ledger(self: &Arc<Self>, url: &str, label: String) -> Result<()> {
        use crate::net::{JoinRequest, UnbillEndpoint};
        let (ledger_id, host, token) = parse_join_url(url)?;
        let local_label = (!label.trim().is_empty()).then_some(label.trim().to_owned());
        let request = JoinRequest { token, ledger_id };
        let ep = UnbillEndpoint::bind(self.secret_key.clone())
            .await
            .map_err(UnbillError::Other)?;
        let result = ep.join_ledger_inner(host, local_label, request, self).await;
        ep.close().await;
        result.map_err(UnbillError::Other)
    }

    /// Fetch a saved user from another device using an `unbill://user/...` URL.
    ///
    /// URL format: `unbill://user/<host_node_id>/<token_hex>`
    pub async fn fetch_local_user(self: &Arc<Self>, url: &str) -> Result<()> {
        use crate::net::UnbillEndpoint;
        let (host, token) = parse_user_url(url)?;
        let ep = UnbillEndpoint::bind(self.secret_key.clone())
            .await
            .map_err(UnbillError::Other)?;
        let result = ep.import_user_inner(host, token, self).await;
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

    /// Open an endpoint and accept incoming sync/join/user-transfer connections until
    /// an error occurs or the process is interrupted.
    ///
    /// Prints the local `NodeId` to stdout so peers know what to dial.
    pub async fn accept_loop(self: &Arc<Self>) -> Result<()> {
        use crate::net::UnbillEndpoint;
        let ep = UnbillEndpoint::bind(self.secret_key.clone())
            .await
            .map_err(UnbillError::Other)?;
        ep.wait_for_ready().await;
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
    // Local users
    // -----------------------------------------------------------------------

    /// Return all saved users stored on this device.
    pub async fn list_local_users(&self) -> Result<Vec<LocalUser>> {
        load_local_users(&*self.store).await
    }

    /// Add a new saved user (fresh user ID + display name) to this device.
    pub async fn add_local_user(&self, display_name: String) -> Result<LocalUser> {
        let local_user = LocalUser {
            user_id: Ulid::new(),
            display_name,
        };
        let mut local_users = load_local_users(&*self.store).await?;
        local_users.push(local_user.clone());
        save_local_users(&*self.store, &local_users).await?;
        Ok(local_user)
    }

    /// Remove a saved user from this device's local storage (does not affect the ledger).
    pub async fn remove_local_user(&self, user_id: Ulid) -> Result<()> {
        let mut local_users = load_local_users(&*self.store).await?;
        let before = local_users.len();
        local_users.retain(|i| i.user_id != user_id);
        if local_users.len() == before {
            return Err(UnbillError::Other(anyhow::anyhow!(
                "local user {user_id} not found"
            )));
        }
        save_local_users(&*self.store, &local_users).await
    }

    /// Import an existing saved user onto this device.
    pub async fn import_local_user(
        &self,
        user_id: Ulid,
        display_name: String,
    ) -> Result<LocalUser> {
        let local_user = LocalUser {
            user_id,
            display_name,
        };
        let mut local_users = load_local_users(&*self.store).await?;
        if !local_users.iter().any(|i| i.user_id == local_user.user_id) {
            local_users.push(local_user.clone());
            save_local_users(&*self.store, &local_users).await?;
        }
        Ok(local_user)
    }

    // -----------------------------------------------------------------------
    // Internals
    // -----------------------------------------------------------------------

    async fn load_doc(&self, ledger_id: &str) -> Result<LedgerDoc> {
        let bytes = self.store.load_ledger_bytes(ledger_id).await?;
        if bytes.is_empty() {
            return Err(UnbillError::LedgerNotFound(ledger_id.to_string()));
        }
        LedgerDoc::from_bytes(&bytes).map_err(UnbillError::Other)
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
// Local user type
// ---------------------------------------------------------------------------

/// A saved user stored on this device.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct LocalUser {
    pub user_id: Ulid,
    pub display_name: String,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const LOCAL_USERS_KEY: &str = "users.json";
pub(crate) const DEVICE_LABELS_KEY: &str = "device_labels.json";
const PENDING_USER_TOKENS_KEY: &str = "pending_user_tokens.json";

async fn load_local_users(store: &dyn LedgerStore) -> Result<Vec<LocalUser>> {
    match store.load_device_meta(LOCAL_USERS_KEY).await? {
        None => Ok(vec![]),
        Some(bytes) => serde_json::from_slice(&bytes)
            .map_err(|e| UnbillError::Other(anyhow::anyhow!("users.json: {e}"))),
    }
}

async fn save_local_users(store: &dyn LedgerStore, local_users: &[LocalUser]) -> Result<()> {
    let bytes = serde_json::to_vec(local_users)
        .map_err(|e| UnbillError::Other(anyhow::anyhow!("serialize users: {e}")))?;
    store.save_device_meta(LOCAL_USERS_KEY, &bytes).await?;
    Ok(())
}

pub(crate) async fn load_device_labels(store: &dyn LedgerStore) -> Result<HashMap<String, String>> {
    match store.load_device_meta(DEVICE_LABELS_KEY).await? {
        None => Ok(HashMap::new()),
        Some(bytes) => serde_json::from_slice(&bytes)
            .map_err(|e| UnbillError::Other(anyhow::anyhow!("device_labels.json: {e}"))),
    }
}

pub(crate) async fn save_device_labels(
    store: &dyn LedgerStore,
    labels: &HashMap<String, String>,
) -> Result<()> {
    let bytes = serde_json::to_vec(labels)
        .map_err(|e| UnbillError::Other(anyhow::anyhow!("serialize device_labels: {e}")))?;
    store.save_device_meta(DEVICE_LABELS_KEY, &bytes).await?;
    Ok(())
}

pub(crate) const PENDING_INVITATIONS_KEY: &str = "pending_invitations.json";

pub(crate) async fn load_pending_invitations(
    store: &dyn LedgerStore,
) -> Result<HashMap<String, Invitation>> {
    match store.load_device_meta(PENDING_INVITATIONS_KEY).await? {
        None => Ok(HashMap::new()),
        Some(bytes) => serde_json::from_slice(&bytes)
            .map_err(|e| UnbillError::Other(anyhow::anyhow!("pending_invitations.json: {e}"))),
    }
}

pub(crate) async fn save_pending_invitations(
    store: &dyn LedgerStore,
    map: &HashMap<String, Invitation>,
) -> Result<()> {
    let bytes = serde_json::to_vec(map)
        .map_err(|e| UnbillError::Other(anyhow::anyhow!("serialize pending_invitations: {e}")))?;
    store
        .save_device_meta(PENDING_INVITATIONS_KEY, &bytes)
        .await?;
    Ok(())
}

pub(crate) async fn load_pending_user_tokens(
    store: &dyn LedgerStore,
) -> Result<HashMap<String, (Ulid, String)>> {
    match store.load_device_meta(PENDING_USER_TOKENS_KEY).await? {
        None => Ok(HashMap::new()),
        Some(bytes) => serde_json::from_slice(&bytes)
            .map_err(|e| UnbillError::Other(anyhow::anyhow!("pending_user_tokens.json: {e}"))),
    }
}

pub(crate) async fn save_pending_user_tokens(
    store: &dyn LedgerStore,
    map: &HashMap<String, (Ulid, String)>,
) -> Result<()> {
    let bytes = serde_json::to_vec(map)
        .map_err(|e| UnbillError::Other(anyhow::anyhow!("serialize pending_user_tokens: {e}")))?;
    store
        .save_device_meta(PENDING_USER_TOKENS_KEY, &bytes)
        .await?;
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
        rand::rngs::SysRng
            .try_fill_bytes(&mut arr)
            .expect("system RNG should generate device keys");
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

/// Parse `unbill://user/<host_node_id>/<token_hex>`.
fn parse_user_url(url: &str) -> Result<(NodeId, String)> {
    let path = url
        .strip_prefix("unbill://user/")
        .ok_or_else(|| UnbillError::Other(anyhow::anyhow!("invalid user URL: {url:?}")))?;
    let parts: Vec<&str> = path.splitn(2, '/').collect();
    if parts.len() != 2 {
        return Err(UnbillError::Other(anyhow::anyhow!(
            "invalid user URL (expected host_node_id/token): {url:?}"
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
            amount_cents,
            description: desc.to_owned(),
            payers: vec![Share { user_id: Ulid::from_u128(1), shares: 1 }],
            payees: vec![
                Share { user_id: Ulid::from_u128(1), shares: 1 },
                Share { user_id: Ulid::from_u128(2), shares: 1 },
            ],
            prev: vec![],
        };
        (ledger_id.to_owned(), bill)
    }

    async fn seed_users(svc: &UnbillService, ledger_id: &str) {
        for (n, name) in [(1u128, "Alice"), (2, "Bob")] {
            svc.add_user(
                ledger_id,
                NewUser {
                    user_id: Ulid::from_u128(n),
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
        seed_users(&svc, &lid).await;
        let (_, bill) = two_way_bill("Dinner", 6000, &lid);
        let bill_id = svc.add_bill(&lid, bill).await.unwrap();

        let bills = svc.list_bills(&lid).await.unwrap();
        assert_eq!(bills.0.len(), 1);
        assert_eq!(bills.0[0].id.to_string(), bill_id);
        assert_eq!(bills.0[0].amount_cents, 6000);
    }

    #[tokio::test]
    async fn test_amend_bill_supersedes_original() {
        let svc = open().await;
        let lid = svc
            .create_ledger("Test".into(), usd().into())
            .await
            .unwrap();
        seed_users(&svc, &lid).await;
        let (_, bill) = two_way_bill("Lunch", 3000, &lid);
        let original_id = svc.add_bill(&lid, bill).await.unwrap();

        let amendment_id = svc
            .add_bill(
                &lid,
                NewBill {
                    amount_cents: 4000,
                    description: "Lunch".into(),
                    payers: vec![Share { user_id: Ulid::from_u128(1), shares: 1 }],
                    payees: vec![
                        Share { user_id: Ulid::from_u128(1), shares: 1 },
                        Share { user_id: Ulid::from_u128(2), shares: 1 },
                    ],
                    prev: vec![Ulid::from_string(&original_id).unwrap()],
                },
            )
            .await
            .unwrap();

        let bills = svc.list_bills(&lid).await.unwrap();
        assert_eq!(bills.0.len(), 1, "original should be superseded");
        assert_eq!(bills.0[0].id.to_string(), amendment_id);
        assert_eq!(bills.0[0].amount_cents, 4000);
    }

    // --- settlement ---

    #[tokio::test]
    async fn test_compute_settlement_no_bills_is_empty() {
        let svc = open().await;
        let lid = svc
            .create_ledger("Empty".into(), usd().into())
            .await
            .unwrap();
        seed_users(&svc, &lid).await;
        let alice = Ulid::from_u128(1).to_string();
        let s = svc.compute_settlement_for_user(&alice).await.unwrap();
        assert!(s.transactions.is_empty());
    }

    #[tokio::test]
    async fn test_compute_settlement_cross_ledger() {
        let svc = open().await;

        // Ledger 1: Alice pays $60, Alice+Bob split → Bob owes Alice $30.
        let lid1 = svc.create_ledger("L1".into(), usd().into()).await.unwrap();
        seed_users(&svc, &lid1).await;
        let (_, bill1) = two_way_bill("Rent", 6000, &lid1);
        svc.add_bill(&lid1, bill1).await.unwrap();

        // Ledger 2: Bob pays $20, Alice+Bob split → Alice owes Bob $10.
        let lid2 = svc.create_ledger("L2".into(), usd().into()).await.unwrap();
        seed_users(&svc, &lid2).await;
        let bob_pays = NewBill {
            amount_cents: 2000,
            description: "Utilities".into(),
            payers: vec![Share { user_id: Ulid::from_u128(2), shares: 1 }],
            payees: vec![
                Share { user_id: Ulid::from_u128(1), shares: 1 },
                Share { user_id: Ulid::from_u128(2), shares: 1 },
            ],
            prev: vec![],
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
            seed_users(&svc, &lid).await;
            let (_, bill) = two_way_bill("Rent", 120000, &lid);
            svc.add_bill(&lid, bill).await.unwrap();
            lid
        };
        // Re-open with the same store (simulates a restart).
        let svc2 = UnbillService::open(Arc::clone(&store)).await.unwrap();
        let bills = svc2.list_bills(&lid).await.unwrap();
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
            svc.set_device_label(peer, "Kitchen iPad".into())
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

    // --- local users ---

    #[tokio::test]
    async fn test_add_local_user_appears_in_list() {
        let svc = open().await;
        let local_user = svc.add_local_user("Alice".into()).await.unwrap();
        let list = svc.list_local_users().await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].user_id, local_user.user_id);
        assert_eq!(list[0].display_name, "Alice");
    }

    #[tokio::test]
    async fn test_multiple_local_users_stored() {
        let svc = open().await;
        svc.add_local_user("Alice".into()).await.unwrap();
        svc.add_local_user("Bob".into()).await.unwrap();
        let list = svc.list_local_users().await.unwrap();
        assert_eq!(list.len(), 2);
    }

    #[tokio::test]
    async fn test_import_local_user_deduplicates() {
        let svc = open().await;
        let id = svc.add_local_user("Alice".into()).await.unwrap();
        svc.import_local_user(id.user_id, "Alice".into())
            .await
            .unwrap();
        assert_eq!(svc.list_local_users().await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_local_users_survive_restart() {
        let store = mem_store();
        let user_id = {
            let svc = UnbillService::open(Arc::clone(&store)).await.unwrap();
            svc.add_local_user("Alice".into()).await.unwrap().user_id
        };
        let svc2 = UnbillService::open(Arc::clone(&store)).await.unwrap();
        let list = svc2.list_local_users().await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].user_id, user_id);
    }
}
