//! FFI surface bridging `UnbillConsole` to Swift via UniFFI.
//!
//! Mirrors the aggregated command granularity of the Tauri bridge (bootstrap /
//! ledger-detail / mutations) rather than the raw console methods — fewer, UI-
//! shaped crossings. The DTO assembly is ported from `unbill-tauri`; the two
//! should eventually share an `unbill-shell` crate to avoid drift.
//!
//! Methods are synchronous and `block_on` a stored tokio runtime; the caller
//! (Swift) must invoke them off any tokio thread. The `ServiceEvent` stream is
//! delivered through a callback interface driven by a spawned forwarding task.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::PathBuf;
use std::sync::Arc;

use unbill_asymmetric_channel::AsymChannel;
use unbill_asymmetric_channel::local::LocalAsymChannel;
use unbill_console::event::ServiceEvent;
use unbill_console::model::{
    Bill, BillId, Currency, EffectiveBills, LedgerId, LedgerMeta, NewBill, NewLedger, NewUser,
    NewUserName, NodeId, Share, User, UserId,
};
use unbill_console::service::{ConflictGroup, UnbillConsole};
use unbill_store_fs::FsStore;

uniffi::setup_scaffolding!();

// ---------- Errors ----------

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum FfiError {
    #[error("{0}")]
    Message(String),
}

fn err(e: impl std::fmt::Display) -> FfiError {
    FfiError::Message(e.to_string())
}

// ---------- DTOs (mirror unbill-tauri) ----------

#[derive(uniffi::Record)]
pub struct FfiLedgerSummary {
    pub ledger_id: String,
    pub name: String,
    pub currency: String,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
    pub user_count: u32,
    pub user_names: Vec<String>,
    pub latest_bill_at_ms: Option<i64>,
}

#[derive(uniffi::Record)]
pub struct FfiUser {
    pub user_id: String,
    pub display_name: String,
    pub added_at_ms: i64,
}

#[derive(uniffi::Record)]
pub struct FfiShare {
    pub user_id: String,
    pub shares: u32,
    pub display_name: String,
}

#[derive(uniffi::Record)]
pub struct FfiBill {
    pub id: String,
    pub amount_cents: i64,
    pub description: String,
    pub created_at_ms: i64,
    pub payers: Vec<FfiShare>,
    pub payees: Vec<FfiShare>,
    pub prev: Vec<String>,
}

#[derive(uniffi::Record)]
pub struct FfiTransaction {
    pub from_name: String,
    pub to_name: String,
    pub amount_cents: i64,
}

#[derive(uniffi::Record)]
pub struct FfiConflictGroup {
    pub conflicting: Vec<FfiBill>,
    pub ancestors: Vec<FfiBill>,
}

#[derive(uniffi::Record)]
pub struct FfiSyncDevice {
    pub node_id: String,
    pub label: String,
    pub ledger_names: Vec<String>,
}

#[derive(uniffi::Record)]
pub struct FfiBootstrap {
    pub device_id: String,
    pub ledgers: Vec<FfiLedgerSummary>,
    pub all_users: Vec<FfiUser>,
    pub devices: Vec<FfiSyncDevice>,
}

#[derive(uniffi::Record)]
pub struct FfiLedgerDetail {
    pub summary: FfiLedgerSummary,
    pub users: Vec<FfiUser>,
    pub devices: Vec<FfiSyncDevice>,
    pub bills: Vec<FfiBill>,
    pub conflicts: Vec<FfiConflictGroup>,
    pub settlement: Vec<FfiTransaction>,
}

/// Input for save_bill (a participant's share weight).
#[derive(uniffi::Record)]
pub struct FfiShareInput {
    pub user_id: String,
    pub shares: u32,
}

// ---------- Event stream ----------

#[derive(uniffi::Enum)]
pub enum FfiServiceEvent {
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
    /// The broadcast receiver lagged; the UI should refresh everything.
    ResyncNeeded,
}

impl From<ServiceEvent> for FfiServiceEvent {
    fn from(e: ServiceEvent) -> Self {
        match e {
            ServiceEvent::LedgerUpdated { ledger_id } => Self::LedgerUpdated { ledger_id },
            ServiceEvent::PeerConnected { ledger_id, peer } => {
                Self::PeerConnected { ledger_id, peer }
            }
            ServiceEvent::PeerDisconnected { ledger_id, peer } => {
                Self::PeerDisconnected { ledger_id, peer }
            }
            ServiceEvent::SyncError {
                ledger_id,
                peer,
                error,
            } => Self::SyncError {
                ledger_id,
                peer,
                error,
            },
        }
    }
}

#[uniffi::export(callback_interface)]
pub trait FfiConsoleObserver: Send + Sync {
    /// Called from a tokio worker thread — keep it cheap (e.g. yield to a
    /// Swift AsyncStream); do NOT call back into FfiConsole here (block_on).
    fn on_event(&self, event: FfiServiceEvent);
}

/// Handle to an active event subscription; dropping it stops delivery.
#[derive(uniffi::Object)]
pub struct FfiSubscription {
    handle: tokio::task::AbortHandle,
}

#[uniffi::export]
impl FfiSubscription {
    pub fn cancel(&self) {
        self.handle.abort();
    }
}

impl Drop for FfiSubscription {
    fn drop(&mut self) {
        self.handle.abort();
    }
}

// ---------- Console handle ----------

#[derive(uniffi::Object)]
pub struct FfiConsole {
    rt: tokio::runtime::Runtime,
    inner: Arc<UnbillConsole>,
}

#[uniffi::export]
impl FfiConsole {
    /// Open a colocated device+console rooted at `dir` (the app's data dir).
    #[uniffi::constructor]
    pub fn open(dir: String) -> Result<Arc<Self>, FfiError> {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .map_err(err)?;
        let inner = rt.block_on(async {
            let store = Arc::new(FsStore::open(PathBuf::from(dir)).map_err(err)?);
            let channel = LocalAsymChannel::open(store).await.map_err(err)?;
            // Start the P2P accept loop so peers can connect (join + sync).
            let accept = Arc::clone(&channel);
            tokio::spawn(async move {
                let _ = accept.accept_loop().await;
            });
            Ok::<_, FfiError>(UnbillConsole::open(channel as Arc<dyn AsymChannel>).await)
        })?;
        Ok(Arc::new(Self { rt, inner }))
    }

    pub fn device_id(&self) -> String {
        self.inner.device_id().to_string()
    }

    /// Ledger list + all known users + peer devices.
    pub fn bootstrap(&self) -> Result<FfiBootstrap, FfiError> {
        self.rt.block_on(bootstrap(&self.inner)).map_err(err)
    }

    /// One ledger's users, devices, bills, conflicts, and settlement.
    pub fn ledger_detail(&self, ledger_id: String) -> Result<FfiLedgerDetail, FfiError> {
        let lid = parse_ledger_id(&ledger_id)?;
        self.rt
            .block_on(ledger_detail(&self.inner, lid))
            .map_err(err)
    }

    pub fn create_ledger(
        &self,
        name: String,
        currency: String,
    ) -> Result<FfiLedgerSummary, FfiError> {
        let currency = Currency::from_code(&currency)
            .ok_or_else(|| FfiError::Message(format!("unknown currency code: {currency}")))?;
        self.rt
            .block_on(async {
                let id = self
                    .inner
                    .create_ledger(NewLedger { name, currency })
                    .await?;
                let meta = find_meta(&self.inner, id).await?;
                summarize_ledger(&self.inner, meta).await
            })
            .map_err(err)
    }

    /// Create a brand-new named user in a ledger.
    pub fn create_user(
        &self,
        ledger_id: String,
        display_name: String,
    ) -> Result<FfiUser, FfiError> {
        let lid = parse_ledger_id(&ledger_id)?;
        self.rt
            .block_on(self.inner.create_user(lid, NewUserName { display_name }))
            .map(user_to_dto)
            .map_err(err)
    }

    /// Add an already-known user (by id) to a ledger.
    pub fn add_user(&self, ledger_id: String, user_id: String) -> Result<FfiUser, FfiError> {
        let lid = parse_ledger_id(&ledger_id)?;
        let uid = parse_user_id(&user_id)?;
        self.rt
            .block_on(add_user(&self.inner, lid, uid))
            .map_err(err)
    }

    /// Save a new bill or an amendment (supersedes `prev_bill_ids`). Returns the new bill id.
    pub fn save_bill(
        &self,
        ledger_id: String,
        amount_cents: i64,
        description: String,
        payers: Vec<FfiShareInput>,
        payees: Vec<FfiShareInput>,
        prev_bill_ids: Vec<String>,
    ) -> Result<String, FfiError> {
        let lid = parse_ledger_id(&ledger_id)?;
        let payers = to_shares(payers)?;
        let payees = to_shares(payees)?;
        let prev = prev_bill_ids
            .iter()
            .map(|s| parse_bill_id(s))
            .collect::<Result<Vec<_>, _>>()?;
        self.rt
            .block_on(self.inner.add_bill(
                lid,
                NewBill {
                    amount_cents,
                    description,
                    payers,
                    payees,
                    prev,
                },
            ))
            .map(|id| id.to_string())
            .map_err(err)
    }

    /// Resolve a conflict group: keep `selected_bill_id`, supersede all `conflicting_bill_ids`.
    pub fn resolve_conflict(
        &self,
        ledger_id: String,
        selected_bill_id: String,
        conflicting_bill_ids: Vec<String>,
    ) -> Result<String, FfiError> {
        let lid = parse_ledger_id(&ledger_id)?;
        let selected = parse_bill_id(&selected_bill_id)?;
        let requested = conflicting_bill_ids
            .iter()
            .map(|s| parse_bill_id(s))
            .collect::<Result<BTreeSet<_>, _>>()?;
        self.rt
            .block_on(resolve_conflict(&self.inner, lid, selected, requested))
            .map_err(err)
    }

    pub fn create_invitation(&self, ledger_id: String) -> Result<String, FfiError> {
        let lid = parse_ledger_id(&ledger_id)?;
        self.rt
            .block_on(self.inner.create_invitation(lid))
            .map_err(err)
    }

    pub fn join_ledger(&self, url: String, label: Option<String>) -> Result<(), FfiError> {
        self.rt
            .block_on(self.inner.join_ledger(&url, label))
            .map_err(err)
    }

    pub fn sync_once(&self, peer_node_id: String) -> Result<(), FfiError> {
        let peer = peer_node_id.parse::<NodeId>().map_err(err)?;
        self.rt.block_on(self.inner.sync_once(peer)).map_err(err)
    }

    /// Subscribe to service events. Delivery stops when the returned handle drops.
    pub fn observe(&self, observer: Box<dyn FfiConsoleObserver>) -> Arc<FfiSubscription> {
        use tokio::sync::broadcast::error::RecvError;
        let mut rx = self.inner.subscribe();
        let handle = self
            .rt
            .spawn(async move {
                loop {
                    match rx.recv().await {
                        Ok(ev) => observer.on_event(ev.into()),
                        Err(RecvError::Lagged(_)) => {
                            observer.on_event(FfiServiceEvent::ResyncNeeded)
                        }
                        Err(RecvError::Closed) => break,
                    }
                }
            })
            .abort_handle();
        Arc::new(FfiSubscription { handle })
    }
}

// ---------- Assembly (ported from unbill-tauri) ----------

type R<T> = Result<T, unbill_console::error::UnbillError>;

async fn bootstrap(service: &Arc<UnbillConsole>) -> R<FfiBootstrap> {
    let mut ledgers = Vec::new();
    for meta in service.list_ledgers().await? {
        ledgers.push(summarize_ledger(service, meta).await?);
    }
    let all_users = service
        .list_all_users()
        .await?
        .into_iter()
        .map(user_to_dto)
        .collect();
    let devices = load_sync_devices(service).await?;
    Ok(FfiBootstrap {
        device_id: service.device_id().to_string(),
        ledgers,
        all_users,
        devices,
    })
}

async fn ledger_detail(service: &Arc<UnbillConsole>, ledger_id: LedgerId) -> R<FfiLedgerDetail> {
    let meta = find_meta(service, ledger_id).await?;
    let summary = summarize_ledger(service, meta).await?;
    let local = service.device_id().to_string();
    let labels = service.list_device_labels().await?;
    let devices = devices_for_ledger(service, ledger_id, &summary.name, &local, &labels).await?;
    let users = service.list_users(ledger_id).await?;
    let bills = service.list_bills(ledger_id).await?;
    let conflicts = service.detect_conflicts(ledger_id).await?;

    let names: HashMap<UserId, String> = users
        .iter()
        .map(|u| (u.user_id, u.display_name.clone()))
        .collect();

    let settlement = service
        .settle_ledger(ledger_id)
        .await?
        .transactions
        .into_iter()
        .map(|t| FfiTransaction {
            from_name: names
                .get(&t.from_user_id)
                .cloned()
                .unwrap_or_else(|| t.from_user_id.to_string()),
            to_name: names
                .get(&t.to_user_id)
                .cloned()
                .unwrap_or_else(|| t.to_user_id.to_string()),
            amount_cents: t.amount_cents,
        })
        .collect();

    Ok(FfiLedgerDetail {
        summary,
        users: users.into_iter().map(user_to_dto).collect(),
        devices,
        bills: map_bills(bills, &names),
        conflicts: conflicts
            .into_iter()
            .map(|g: ConflictGroup| FfiConflictGroup {
                conflicting: map_bill_vec(g.conflicting, &names),
                ancestors: map_bill_vec(g.ancestors, &names),
            })
            .collect(),
        settlement,
    })
}

async fn add_user(service: &Arc<UnbillConsole>, lid: LedgerId, uid: UserId) -> R<FfiUser> {
    let existing = service
        .list_all_users()
        .await?
        .into_iter()
        .find(|u| u.user_id == uid)
        .ok_or_else(|| unbill_console::error::UnbillError::UserNotFound(uid.to_string()))?;
    service
        .add_user(
            lid,
            NewUser {
                user_id: uid,
                display_name: existing.display_name,
            },
        )
        .await?;
    let added = service
        .list_users(lid)
        .await?
        .into_iter()
        .find(|u| u.user_id == uid)
        .ok_or_else(|| {
            unbill_console::error::UnbillError::UserNotFound("new user missing after add".into())
        })?;
    Ok(user_to_dto(added))
}

async fn resolve_conflict(
    service: &Arc<UnbillConsole>,
    ledger_id: LedgerId,
    selected: BillId,
    requested: BTreeSet<BillId>,
) -> R<String> {
    use unbill_console::error::UnbillError;
    if !requested.contains(&selected) {
        return Err(UnbillError::Validation(
            "selected bill is not part of the conflict".into(),
        ));
    }
    let group = service
        .detect_conflicts(ledger_id)
        .await?
        .into_iter()
        .find(|g| g.conflicting.iter().map(|b| b.id).collect::<BTreeSet<_>>() == requested)
        .ok_or_else(|| UnbillError::Validation("conflict group is no longer current".into()))?;
    let bill = group
        .conflicting
        .into_iter()
        .find(|b| b.id == selected)
        .ok_or_else(|| {
            UnbillError::Validation("selected bill is no longer a current conflicting bill".into())
        })?;
    let merge = service
        .add_bill(
            ledger_id,
            NewBill {
                amount_cents: bill.amount_cents,
                description: bill.description,
                payers: bill.payers,
                payees: bill.payees,
                prev: requested.into_iter().collect(),
            },
        )
        .await?;
    Ok(merge.to_string())
}

async fn summarize_ledger(service: &Arc<UnbillConsole>, meta: LedgerMeta) -> R<FfiLedgerSummary> {
    let users = service.list_users(meta.ledger_id).await?;
    let bills = service.list_bills(meta.ledger_id).await?;
    let latest_bill_at_ms = bills.iter().map(|b| b.created_at.as_millis()).max();
    Ok(FfiLedgerSummary {
        ledger_id: meta.ledger_id.to_string(),
        name: meta.name,
        currency: meta.currency.code().to_owned(),
        created_at_ms: meta.created_at.as_millis(),
        updated_at_ms: meta.updated_at.as_millis(),
        user_count: users.len() as u32,
        user_names: users.iter().map(|u| u.display_name.clone()).collect(),
        latest_bill_at_ms,
    })
}

async fn find_meta(service: &Arc<UnbillConsole>, ledger_id: LedgerId) -> R<LedgerMeta> {
    service
        .list_ledgers()
        .await?
        .into_iter()
        .find(|m| m.ledger_id == ledger_id)
        .ok_or_else(|| unbill_console::error::UnbillError::LedgerNotFound(ledger_id.to_string()))
}

async fn load_sync_devices(service: &Arc<UnbillConsole>) -> R<Vec<FfiSyncDevice>> {
    let local = service.device_id().to_string();
    let labels = service.list_device_labels().await?;
    let mut by_node = BTreeMap::<String, FfiSyncDevice>::new();
    for meta in service.list_ledgers().await? {
        let name = meta.name.clone();
        for d in devices_for_ledger(service, meta.ledger_id, &name, &local, &labels).await? {
            let entry = by_node
                .entry(d.node_id.clone())
                .or_insert_with(|| FfiSyncDevice {
                    node_id: d.node_id,
                    label: d.label,
                    ledger_names: Vec::new(),
                });
            if !entry.ledger_names.iter().any(|n| n == &name) {
                entry.ledger_names.push(name.clone());
            }
        }
    }
    let mut devices = by_node.into_values().collect::<Vec<_>>();
    devices.sort_by(|a, b| {
        a.label
            .to_lowercase()
            .cmp(&b.label.to_lowercase())
            .then_with(|| a.node_id.cmp(&b.node_id))
    });
    Ok(devices)
}

async fn devices_for_ledger(
    service: &Arc<UnbillConsole>,
    ledger_id: LedgerId,
    ledger_name: &str,
    local_node_id: &str,
    labels: &HashMap<String, String>,
) -> R<Vec<FfiSyncDevice>> {
    let mut devices = service
        .list_devices(ledger_id)
        .await?
        .into_iter()
        .filter_map(|d| {
            let node_id = d.node_id.to_string();
            if node_id == local_node_id {
                return None;
            }
            Some(FfiSyncDevice {
                label: labels.get(&node_id).cloned().unwrap_or_default(),
                node_id,
                ledger_names: vec![ledger_name.to_owned()],
            })
        })
        .collect::<Vec<_>>();
    devices.sort_by(|a, b| {
        a.label
            .to_lowercase()
            .cmp(&b.label.to_lowercase())
            .then_with(|| a.node_id.cmp(&b.node_id))
    });
    Ok(devices)
}

fn map_bills(bills: EffectiveBills, names: &HashMap<UserId, String>) -> Vec<FfiBill> {
    let mut items = bills
        .into_vec()
        .into_iter()
        .map(|b| bill_to_dto(b, names))
        .collect::<Vec<_>>();
    items.sort_by_key(|b| std::cmp::Reverse(b.created_at_ms));
    items
}

fn map_bill_vec(bills: Vec<Bill>, names: &HashMap<UserId, String>) -> Vec<FfiBill> {
    let mut items = bills
        .into_iter()
        .map(|b| bill_to_dto(b, names))
        .collect::<Vec<_>>();
    items.sort_by_key(|b| std::cmp::Reverse(b.created_at_ms));
    items
}

fn bill_to_dto(bill: Bill, names: &HashMap<UserId, String>) -> FfiBill {
    let to_share = |s: Share| FfiShare {
        display_name: names
            .get(&s.user_id)
            .cloned()
            .unwrap_or_else(|| s.user_id.to_string()),
        user_id: s.user_id.to_string(),
        shares: s.shares,
    };
    FfiBill {
        id: bill.id.to_string(),
        amount_cents: bill.amount_cents,
        description: bill.description,
        created_at_ms: bill.created_at.as_millis(),
        payers: bill.payers.into_iter().map(to_share).collect(),
        payees: bill.payees.into_iter().map(to_share).collect(),
        prev: bill.prev.into_iter().map(|p| p.to_string()).collect(),
    }
}

fn user_to_dto(u: User) -> FfiUser {
    FfiUser {
        user_id: u.user_id.to_string(),
        display_name: u.display_name,
        added_at_ms: u.added_at.as_millis(),
    }
}

fn to_shares(items: Vec<FfiShareInput>) -> Result<Vec<Share>, FfiError> {
    items
        .into_iter()
        .map(|i| {
            Ok(Share {
                user_id: parse_user_id(&i.user_id)?,
                shares: i.shares,
            })
        })
        .collect()
}

fn parse_ledger_id(v: &str) -> Result<LedgerId, FfiError> {
    LedgerId::from_string(v).map_err(err)
}

fn parse_user_id(v: &str) -> Result<UserId, FfiError> {
    UserId::from_string(v).map_err(err)
}

fn parse_bill_id(v: &str) -> Result<BillId, FfiError> {
    BillId::from_string(v).map_err(err)
}
