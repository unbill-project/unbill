//! Shared `Console` API bridging `UnbillConsole` to Swift and Python via UniFFI.
//!
//! Mirrors the aggregated command granularity of the Tauri bridge (bootstrap /
//! ledger-detail / mutations) rather than the raw console methods — fewer, UI-
//! shaped crossings. The DTO assembly is ported from `unbill-tauri`; the two
//! should eventually share an `unbill-shell` crate to avoid drift.
//!
//! Methods are synchronous and `block_on` a stored tokio runtime; the caller
//! must invoke them off any tokio thread. The `ServiceEvent` stream is
//! delivered through a callback interface driven by a spawned forwarding task.

build_info::build_info!(fn compiled_build_info);

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::PathBuf;
use std::sync::Arc;

use unbill_asymmetric_channel::local::LocalAsymChannel;
#[cfg(not(any(
    target_os = "android",
    all(target_os = "ios", not(target_abi = "macabi"))
)))]
use unbill_asymmetric_channel::rpc::RpcAsymChannel;
use unbill_console::event::ServiceEvent;
use unbill_console::model::{
    Bill, BillId, Currency, EffectiveBills, LedgerId, LedgerMeta, NewBill, NewLedger, NewUser,
    NewUserName, NodeId, Share, StorageError, User, UserId,
};
use unbill_console::service::{ConflictGroup, UnbillConsole};
use unbill_store_sqlite::SqliteStore;

uniffi::setup_scaffolding!();

// sirno:witness:unbill-license:begin
#[derive(uniffi::Record)]
pub struct FfiLegalSoftware {
    pub name: String,
    pub versions: Vec<String>,
    pub license_expressions: Vec<String>,
}

#[derive(uniffi::Record)]
pub struct FfiLegalDocument {
    pub title: String,
    pub text: String,
}

#[derive(uniffi::Record)]
pub struct FfiLegalPackage {
    pub package_id: String,
    pub name: String,
    pub version: String,
    pub license_expression: String,
    pub repository: Option<String>,
    pub source: Option<String>,
    pub documents: Vec<FfiLegalDocument>,
}

/// Local compiled information; never opens storage or contacts a daemon.
#[uniffi::export]
pub fn license_software_summaries() -> Vec<FfiLegalSoftware> {
    unbill_license::software_summaries()
        .into_iter()
        .map(|software| FfiLegalSoftware {
            name: software.name,
            versions: software.versions,
            license_expressions: software.license_expressions,
        })
        .collect()
}

#[uniffi::export]
pub fn license_software_details(name: String) -> Vec<FfiLegalPackage> {
    unbill_license::software(&name)
        .iter()
        .map(|package| FfiLegalPackage {
            package_id: package.package_id.clone(),
            name: package.name.clone(),
            version: package.version.clone(),
            license_expression: package.license_expression.clone(),
            repository: package.repository.clone(),
            source: package
                .source_archive
                .clone()
                .or_else(|| package.source.clone()),
            documents: unbill_license::display_documents(package)
                .into_iter()
                .map(|document| FfiLegalDocument {
                    title: document.path,
                    text: document.text,
                })
                .collect(),
        })
        .collect()
}
// sirno:witness:unbill-license:end

/// Complete currency catalog for frontend pickers, sorted by ISO code.
#[uniffi::export]
pub fn supported_currency_codes() -> Vec<String> {
    let mut codes: Vec<_> = Currency::all()
        .map(|currency| currency.code().to_owned())
        .collect();
    codes.sort_unstable();
    codes
}

/// Shared desktop data-directory default, including UNBILL_DATA_DIR overrides.
#[uniffi::export]
pub fn default_data_directory() -> Result<String, FfiError> {
    let root = unbill_storage::UNBILL_PATH.data_dir().map_err(err)?;
    root.into_os_string()
        .into_string()
        .map_err(|_| err("data directory is not UTF-8"))
}

#[derive(uniffi::Record)]
pub struct FfiBuildInfo {
    pub version: String,
    pub built_at_utc: String,
}

impl From<unbill_build_info::BuildInfo> for FfiBuildInfo {
    fn from(info: unbill_build_info::BuildInfo) -> Self {
        Self {
            version: info.version,
            built_at_utc: info.built_at_utc,
        }
    }
}

#[uniffi::export]
pub fn client_build_info() -> FfiBuildInfo {
    unbill_build_info::BuildInfo::from(crate::compiled_build_info()).into()
}

// ---------- Errors ----------

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum FfiError {
    #[error("data directory is already in use: {path}")]
    DataDirectoryInUse { path: String },
    #[error("{0}")]
    Message(String),
}

fn err(e: impl std::fmt::Display) -> FfiError {
    FfiError::Message(e.to_string())
}

// ---------- DTOs (mirror unbill-tauri) ----------

#[derive(uniffi::Record)]
pub struct FfiLedgerSummary {
    pub archived: bool,
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
    pub archived: bool,
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
pub struct FfiUserAmount {
    pub user_id: String,
    pub amount_cents: i64,
}

#[derive(uniffi::Record)]
pub struct FfiBillSplit {
    pub payer_amounts: Vec<FfiUserAmount>,
    pub payee_amounts: Vec<FfiUserAmount>,
}

/// Identity of a draft and the bills it supersedes when saved.
#[derive(uniffi::Record)]
pub struct FfiBillDraft {
    pub bill_id: String,
    pub prev_bill_ids: Vec<String>,
}

/// Allocate a real bill ID for previewing and saving a new bill or amendment.
#[uniffi::export]
pub fn new_bill_id() -> String {
    BillId::new().to_string()
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
    pub emoji_fingerprint: String,

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

// sirno:witness:unbill-ffi:begin
#[derive(uniffi::Enum)]
pub enum FfiBleState {
    Disabled,
    Active,
    Unavailable { reason: String },
    Closed,
}
impl From<unbill_console::model::BleState> for FfiBleState {
    fn from(value: unbill_console::model::BleState) -> Self {
        match value {
            unbill_console::model::BleState::Disabled => Self::Disabled,
            unbill_console::model::BleState::Active => Self::Active,
            unbill_console::model::BleState::Unavailable { reason } => Self::Unavailable { reason },
            unbill_console::model::BleState::Closed => Self::Closed,
        }
    }
}
#[derive(uniffi::Enum)]
pub enum FfiBlePeerPhase {
    Discovered,
    PendingDial,
    Connecting,
    Handshaking,
    Connected,
    Draining,
    Reconnecting,
    Restoring,
    Unknown,
}
impl From<unbill_console::model::BlePeerPhase> for FfiBlePeerPhase {
    fn from(value: unbill_console::model::BlePeerPhase) -> Self {
        match value {
            unbill_console::model::BlePeerPhase::Discovered => Self::Discovered,
            unbill_console::model::BlePeerPhase::PendingDial => Self::PendingDial,
            unbill_console::model::BlePeerPhase::Connecting => Self::Connecting,
            unbill_console::model::BlePeerPhase::Handshaking => Self::Handshaking,
            unbill_console::model::BlePeerPhase::Connected => Self::Connected,
            unbill_console::model::BlePeerPhase::Draining => Self::Draining,
            unbill_console::model::BlePeerPhase::Reconnecting => Self::Reconnecting,
            unbill_console::model::BlePeerPhase::Restoring => Self::Restoring,
            unbill_console::model::BlePeerPhase::Unknown => Self::Unknown,
        }
    }
}
#[derive(uniffi::Enum)]
pub enum FfiBleConnectPath {
    Gatt,
    L2cap,
}
impl From<unbill_console::model::BleConnectPath> for FfiBleConnectPath {
    fn from(value: unbill_console::model::BleConnectPath) -> Self {
        match value {
            unbill_console::model::BleConnectPath::Gatt => Self::Gatt,
            unbill_console::model::BleConnectPath::L2cap => Self::L2cap,
        }
    }
}
#[derive(uniffi::Record)]
pub struct FfiBlePeer {
    pub device_id: String,
    pub phase: FfiBlePeerPhase,
    pub consecutive_failures: u32,
    pub connect_path: Option<FfiBleConnectPath>,
    pub verified_node_id: Option<String>,
}

#[derive(uniffi::Record)]
pub struct FfiBleSnapshot {
    pub state: FfiBleState,
    pub peers: Vec<FfiBlePeer>,
    pub discovered_node_ids: Vec<String>,
}

impl From<unbill_console::model::BleSnapshot> for FfiBleSnapshot {
    fn from(value: unbill_console::model::BleSnapshot) -> Self {
        Self {
            state: value.state.into(),
            discovered_node_ids: value
                .discovered_node_ids
                .into_iter()
                .map(|id| id.to_string())
                .collect(),
            peers: value
                .peers
                .into_iter()
                .map(|peer| FfiBlePeer {
                    device_id: peer.device_id,
                    phase: peer.phase.into(),
                    consecutive_failures: peer.consecutive_failures,
                    connect_path: peer.connect_path.map(Into::into),
                    verified_node_id: peer.verified_node_id.map(|id| id.to_string()),
                })
                .collect(),
        }
    }
}

#[uniffi::export(callback_interface)]
pub trait FfiBleObserver: Send + Sync {
    /// Delivered on a worker thread; dispatch UI changes to the main thread.
    fn on_snapshot(&self, snapshot: FfiBleSnapshot);
}
// sirno:witness:unbill-ffi:end

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
            ServiceEvent::DeviceIdentityInitialized | ServiceEvent::PendingInvitationsUpdated => {
                Self::ResyncNeeded
            }
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
    /// Swift AsyncStream); do NOT call back into Console here (block_on).
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

// sirno:witness:unbill-ffi:begin
#[derive(uniffi::Object)]
pub struct Console {
    rt: tokio::runtime::Runtime,
    inner: Arc<UnbillConsole>,
}

#[uniffi::export]
impl Console {
    /// Run a device and console in this process, owning the SQLite data directory.
    #[uniffi::constructor]
    pub fn open_in_process(dir: String) -> Result<Arc<Self>, FfiError> {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .map_err(err)?;
        let inner = rt.block_on(async {
            let store =
                SqliteStore::open(PathBuf::from(&dir))
                    .await
                    .map_err(|error| match error {
                        StorageError::Io(error)
                            if error.kind() == std::io::ErrorKind::WouldBlock =>
                        {
                            FfiError::DataDirectoryInUse { path: dir.clone() }
                        }
                        error => err(error),
                    })?;
            let channel = LocalAsymChannel::open(
                Arc::new(store),
                unbill_build_info::BuildInfo::from(crate::compiled_build_info()),
            )
            .await
            .map_err(err)?;
            let accept = Arc::clone(&channel);
            tokio::spawn(async move {
                let _ = accept.accept_loop().await;
            });
            Ok::<_, FfiError>(UnbillConsole::open(channel).await)
        })?;
        Ok(Arc::new(Self { rt, inner }))
    }
}

#[cfg(not(any(
    target_os = "android",
    all(target_os = "ios", not(target_abi = "macabi"))
)))]
#[uniffi::export]
impl Console {
    /// Connect to an existing daemon socket in `dir` (or UNBILL_SOCKET).
    #[uniffi::constructor]
    pub fn open_rpc(dir: String) -> Result<Arc<Self>, FfiError> {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .map_err(err)?;
        let inner = rt.block_on(async {
            let socket = std::env::var_os("UNBILL_SOCKET")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from(&dir).join("unbill.sock"));
            let channel = RpcAsymChannel::connect(&socket).await.map_err(|error| {
                err(format!(
                    "could not connect to unbill-daemon at {}: {error}; start unbill-daemon first",
                    socket.display()
                ))
            })?;
            Ok::<_, FfiError>(UnbillConsole::open(channel).await)
        })?;
        Ok(Arc::new(Self { rt, inner }))
    }
}
// sirno:witness:unbill-ffi:end

#[uniffi::export]
impl Console {
    pub fn service_build_info(&self) -> Result<FfiBuildInfo, FfiError> {
        self.rt
            .block_on(self.inner.service_build_info())
            .map(FfiBuildInfo::from)
            .map_err(err)
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
    pub fn set_ledger_archived(&self, ledger_id: String, archived: bool) -> Result<(), FfiError> {
        let lid = LedgerId::from_string(&ledger_id).map_err(err)?;
        self.rt
            .block_on(self.inner.set_ledger_archived(lid, archived))
            .map_err(err)
    }

    pub fn set_user_archived(
        &self,
        ledger_id: String,
        user_id: String,
        archived: bool,
    ) -> Result<(), FfiError> {
        let lid = parse_ledger_id(&ledger_id)?;
        let uid = parse_user_id(&user_id)?;
        self.rt
            .block_on(self.inner.set_user_archived(lid, uid, archived))
            .map_err(err)
    }

    pub fn add_user(&self, ledger_id: String, user_id: String) -> Result<FfiUser, FfiError> {
        let lid = parse_ledger_id(&ledger_id)?;
        let uid = parse_user_id(&user_id)?;
        self.rt
            .block_on(add_user(&self.inner, lid, uid))
            .map_err(err)
    }

    /// Calculate a draft using the same verified splitting algorithm as other clients.
    pub fn preview_bill_split(
        &self,
        amount_cents: i64,
        payers: Vec<FfiShareInput>,
        payees: Vec<FfiShareInput>,
        bill_id: String,
    ) -> Result<FfiBillSplit, FfiError> {
        let split = self
            .inner
            .calculate_bill_split(
                &to_shares(payers)?,
                &to_shares(payees)?,
                amount_cents,
                parse_bill_id(&bill_id)?,
            )
            .map_err(err)?;
        let amounts = |items: Vec<(UserId, i64)>| {
            items
                .into_iter()
                .map(|(id, amount_cents)| FfiUserAmount {
                    user_id: id.to_string(),
                    amount_cents,
                })
                .collect()
        };
        Ok(FfiBillSplit {
            payer_amounts: amounts(split.payer_amounts),
            payee_amounts: amounts(split.payee_amounts),
        })
    }

    /// Save a new bill or an amendment (supersedes `prev_bill_ids`). Returns the new bill id.
    pub fn save_bill(
        &self,
        ledger_id: String,
        amount_cents: i64,
        description: String,
        payers: Vec<FfiShareInput>,
        payees: Vec<FfiShareInput>,
        draft: FfiBillDraft,
    ) -> Result<String, FfiError> {
        let lid = parse_ledger_id(&ledger_id)?;
        let bill_id = parse_bill_id(&draft.bill_id)?;
        let payers = to_shares(payers)?;
        let payees = to_shares(payees)?;
        let prev = draft
            .prev_bill_ids
            .iter()
            .map(|s| parse_bill_id(s))
            .collect::<Result<Vec<_>, _>>()?;
        self.rt
            .block_on(self.inner.add_bill_with_id(
                lid,
                NewBill {
                    amount_cents,
                    description,
                    payers,
                    payees,
                    prev,
                },
                bill_id,
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

    pub fn join_ledger(&self, url: String, label: String) -> Result<(), FfiError> {
        self.rt
            .block_on(self.inner.join_ledger(
                &url,
                unbill_console::model::DeviceLabel::new(label).map_err(err)?,
            ))
            .map_err(err)
    }

    /// Await network sync without blocking the caller's Swift actor executor.
    pub async fn sync_once(&self, peer_node_id: String) -> Result<(), FfiError> {
        let peer = peer_node_id.parse::<NodeId>().map_err(err)?;
        let inner = Arc::clone(&self.inner);
        self.rt
            .spawn(async move { inner.sync_once(peer).await })
            .await
            .map_err(err)?
            .map_err(err)
    }

    /// Subscribe to BLE state, including an initial snapshot.
    /// Cancel or drop the returned subscription to stop callbacks.
    pub async fn observe_ble(
        &self,
        observer: Box<dyn FfiBleObserver>,
    ) -> Result<Arc<FfiSubscription>, FfiError> {
        let inner = Arc::clone(&self.inner);
        let mut rx = self
            .rt
            .spawn(async move { inner.subscribe_ble().await })
            .await
            .map_err(err)?
            .map_err(err)?;
        let handle = self
            .rt
            .spawn(async move {
                loop {
                    let snapshot = rx.borrow_and_update().clone();
                    observer.on_snapshot(snapshot.into());
                    if rx.changed().await.is_err() {
                        break;
                    }
                }
            })
            .abort_handle();
        Ok(Arc::new(FfiSubscription { handle }))
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
    let devices = devices_for_ledger(service, ledger_id, &summary.name, &local).await?;
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
        emoji_fingerprint: service.ledger_emojis(ledger_id).await?,
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
        archived: meta.archived,
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
    let mut by_node = BTreeMap::<String, FfiSyncDevice>::new();
    let mut metas = service.list_ledgers().await?;
    metas.sort_by_key(|meta| meta.ledger_id);
    for meta in metas {
        let name = meta.name.clone();
        for d in devices_for_ledger(service, meta.ledger_id, &name, &local).await? {
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
                label: d.label.to_string(),
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
        archived: u.archived,
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

#[cfg(test)]
mod tests {
    #[test]
    fn ble_observer_receives_initial_snapshot_and_releases_on_cancel() {
        struct Observer(std::sync::mpsc::Sender<FfiBleSnapshot>);
        impl FfiBleObserver for Observer {
            fn on_snapshot(&self, snapshot: FfiBleSnapshot) {
                let _ = self.0.send(snapshot);
            }
        }
        let dir = tempfile::tempdir().unwrap();
        let console = Console::open_in_process(dir.path().to_str().unwrap().to_owned()).unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        let subscription = console
            .rt
            .block_on(console.observe_ble(Box::new(Observer(tx))))
            .unwrap();
        let snapshot = rx.recv_timeout(std::time::Duration::from_secs(2)).unwrap();
        assert!(matches!(snapshot.state, FfiBleState::Disabled));
        assert!(snapshot.peers.is_empty());
        subscription.cancel();
        assert!(matches!(
            rx.recv_timeout(std::time::Duration::from_secs(2)),
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected)
        ));
    }

    #[test]
    fn legal_catalog_and_original_project_texts_are_available_without_console() {
        let summaries = super::license_software_summaries();
        assert_eq!(summaries.first().unwrap().name, "unbill");
        let details = super::license_software_details("unbill".to_owned());
        let project = details.first().unwrap();
        assert_eq!(project.license_expression, "MIT OR Apache-2.0");
        assert!(
            project
                .documents
                .iter()
                .any(|document| document.text == include_str!("../../../LICENSE-MIT"))
        );
        assert!(
            project
                .documents
                .iter()
                .any(|document| document.text == include_str!("../../../LICENSE-APACHE"))
        );
        assert!(super::license_software_details("unknown-software".to_owned()).is_empty());
    }
    use super::*;

    #[cfg(not(any(
        target_os = "android",
        all(target_os = "ios", not(target_abi = "macabi"))
    )))]
    struct TestDaemon {
        stop: Option<tokio::sync::oneshot::Sender<()>>,
        thread: Option<std::thread::JoinHandle<()>>,
    }

    #[cfg(not(any(
        target_os = "android",
        all(target_os = "ios", not(target_abi = "macabi"))
    )))]
    impl TestDaemon {
        fn start(root: PathBuf) -> Self {
            let (stop, stopped) = tokio::sync::oneshot::channel();
            let (ready, wait_ready) = std::sync::mpsc::channel();
            let thread = std::thread::spawn(move || {
                let rt = tokio::runtime::Builder::new_multi_thread()
                    .enable_all()
                    .build()
                    .unwrap();
                rt.block_on(async move {
                    let store = Arc::new(SqliteStore::open(root.clone()).await.unwrap());
                    let channel = LocalAsymChannel::open(
                        store,
                        unbill_build_info::BuildInfo {
                            version: "test-daemon".to_owned(),
                            built_at_utc: "2025-01-01T00:00:00Z".to_owned(),
                        },
                    )
                    .await
                    .unwrap();
                    let socket = root.join("unbill.sock");
                    let path = socket.clone();
                    let serving = tokio::spawn(async move {
                        unbill_asymmetric_channel::rpc::serve(channel, &path).await
                    });
                    tokio::time::timeout(std::time::Duration::from_secs(5), async {
                        while !socket.exists() {
                            assert!(!serving.is_finished(), "RPC listener failed to start");
                            tokio::task::yield_now().await;
                        }
                    })
                    .await
                    .unwrap();
                    ready.send(()).unwrap();
                    stopped.await.unwrap();
                    serving.abort();
                    let _ = serving.await;
                });
            });
            wait_ready
                .recv_timeout(std::time::Duration::from_secs(10))
                .unwrap();
            Self {
                stop: Some(stop),
                thread: Some(thread),
            }
        }
    }
    #[cfg(not(any(
        target_os = "android",
        all(target_os = "ios", not(target_abi = "macabi"))
    )))]
    impl Drop for TestDaemon {
        fn drop(&mut self) {
            let _ = self.stop.take().unwrap().send(());
            self.thread.take().unwrap().join().unwrap();
        }
    }

    #[cfg(not(any(
        target_os = "android",
        all(target_os = "ios", not(target_abi = "macabi"))
    )))]
    #[test]
    fn desktop_bridge_reports_missing_daemon_without_creating_storage() {
        let parent = tempfile::tempdir().unwrap();
        let root = parent.path().join("absent");
        let error = match Console::open_rpc(root.to_str().unwrap().to_owned()) {
            Err(error) => error,
            Ok(_) => panic!("desktop must connect to an existing daemon"),
        };
        assert!(
            error
                .to_string()
                .contains("could not connect to unbill-daemon")
        );
        assert!(!root.exists());
    }

    #[test]
    fn sqlite_identity_and_ledger_survive_in_process_reopening() {
        let dir = tempfile::tempdir().unwrap();
        assert_sqlite_round_trip(dir.path(), Console::open_in_process);
    }

    #[cfg(not(any(
        target_os = "android",
        all(target_os = "ios", not(target_abi = "macabi"))
    )))]
    #[test]
    fn sqlite_identity_and_ledger_survive_daemon_reopening() {
        let dir = tempfile::tempdir().unwrap();
        let _daemon = TestDaemon::start(dir.path().into());
        assert_sqlite_round_trip(dir.path(), Console::open_rpc);
    }

    #[cfg(not(any(
        target_os = "android",
        all(target_os = "ios", not(target_abi = "macabi"))
    )))]
    #[test]
    fn daemon_consoles_share_device_state_without_owning_storage() {
        let dir = tempfile::tempdir().unwrap();
        let _daemon = TestDaemon::start(dir.path().into());
        let path = dir.path().to_str().unwrap().to_owned();
        let first = Console::open_rpc(path.clone()).unwrap();
        let second = Console::open_rpc(path.clone()).unwrap();
        let service = first.service_build_info().unwrap();
        assert_eq!(service.version, "test-daemon");
        assert_eq!(service.built_at_utc, "2025-01-01T00:00:00Z");
        assert_eq!(first.device_id(), second.device_id());
        let ledger = first
            .create_ledger("Shared daemon".into(), "USD".into())
            .unwrap();
        assert_eq!(
            second.ledger_detail(ledger.ledger_id).unwrap().summary.name,
            "Shared daemon"
        );
        assert!(matches!(
            Console::open_in_process(path),
            Err(FfiError::DataDirectoryInUse { .. })
        ));
    }

    #[cfg(not(any(
        target_os = "android",
        all(target_os = "ios", not(target_abi = "macabi"))
    )))]
    #[test]
    fn in_process_console_releases_directory_ownership_on_drop() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().to_str().unwrap().to_owned();
        let first = Console::open_in_process(path.clone()).unwrap();
        assert_eq!(
            first.service_build_info().unwrap().version,
            client_build_info().version
        );
        assert!(matches!(
            Console::open_in_process(path.clone()),
            Err(FfiError::DataDirectoryInUse { .. })
        ));
        drop(first);
        let reopened = Console::open_in_process(path).unwrap();
        assert!(reopened.bootstrap().unwrap().ledgers.is_empty());
    }

    fn assert_sqlite_round_trip(
        root: &std::path::Path,
        open: fn(String) -> Result<Arc<Console>, FfiError>,
    ) {
        let path = root.to_str().unwrap().to_owned();
        let console = open(path.clone()).unwrap();
        let device_id = console.device_id();
        let ledger = console
            .create_ledger("Shared dinner".into(), "USD".into())
            .unwrap();
        let alice = console
            .create_user(ledger.ledger_id.clone(), "Alice".into())
            .unwrap();
        let bob = console
            .create_user(ledger.ledger_id.clone(), "Bob".into())
            .unwrap();
        let payer_shares = || {
            vec![
                FfiShareInput {
                    user_id: alice.user_id.clone(),
                    shares: 1,
                },
                FfiShareInput {
                    user_id: bob.user_id.clone(),
                    shares: 3,
                },
            ]
        };
        let payee_shares = || {
            vec![
                FfiShareInput {
                    user_id: alice.user_id.clone(),
                    shares: 1,
                },
                FfiShareInput {
                    user_id: bob.user_id.clone(),
                    shares: 1,
                },
            ]
        };
        let seed = new_bill_id();
        let preview = console
            .preview_bill_split(4000, payer_shares(), payee_shares(), seed.clone())
            .unwrap();
        assert_eq!(
            preview
                .payer_amounts
                .iter()
                .map(|a| a.amount_cents)
                .collect::<Vec<_>>(),
            vec![1000, 3000]
        );
        assert_eq!(
            preview
                .payee_amounts
                .iter()
                .map(|a| a.amount_cents)
                .sum::<i64>(),
            4000
        );
        assert!(
            console
                .ledger_detail(ledger.ledger_id.clone())
                .unwrap()
                .bills
                .is_empty()
        );
        assert!(
            console
                .preview_bill_split(100, vec![], payee_shares(), seed.clone())
                .is_err()
        );
        assert!(
            console
                .preview_bill_split(i64::MAX, payer_shares(), payee_shares(), seed.clone())
                .is_err()
        );
        let rounded_preview = console
            .preview_bill_split(4001, payer_shares(), payee_shares(), seed.clone())
            .unwrap();
        let original = console
            .save_bill(
                ledger.ledger_id.clone(),
                4001,
                "Dinner".into(),
                payer_shares(),
                payee_shares(),
                FfiBillDraft {
                    bill_id: seed.clone(),
                    prev_bill_ids: vec![],
                },
            )
            .unwrap();
        assert_eq!(original, seed);
        let saved = console.ledger_detail(ledger.ledger_id.clone()).unwrap();
        let saved_bill = saved.bills.first().unwrap();
        let saved_split = console
            .preview_bill_split(
                saved_bill.amount_cents,
                payer_shares(),
                payee_shares(),
                saved_bill.id.clone(),
            )
            .unwrap();
        let amounts = |entries: Vec<FfiUserAmount>| {
            entries
                .into_iter()
                .map(|entry| (entry.user_id, entry.amount_cents))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            amounts(rounded_preview.payer_amounts),
            amounts(saved_split.payer_amounts)
        );
        assert_eq!(
            amounts(rounded_preview.payee_amounts),
            amounts(saved_split.payee_amounts)
        );
        assert!(
            console
                .save_bill(
                    ledger.ledger_id.clone(),
                    4001,
                    "Duplicate".into(),
                    payer_shares(),
                    payee_shares(),
                    FfiBillDraft {
                        bill_id: seed.clone(),
                        prev_bill_ids: vec![]
                    },
                )
                .is_err()
        );
        let amended = console
            .save_bill(
                ledger.ledger_id.clone(),
                5000,
                "Corrected dinner".into(),
                payer_shares(),
                payee_shares(),
                FfiBillDraft {
                    bill_id: new_bill_id(),
                    prev_bill_ids: vec![original.clone()],
                },
            )
            .unwrap();
        let bills = console
            .ledger_detail(ledger.ledger_id.clone())
            .unwrap()
            .bills;
        assert_eq!(bills.len(), 1);
        assert_eq!(bills.first().unwrap().id, amended);
        assert_eq!(bills.first().unwrap().prev, vec![original]);
        drop(console);

        let console = open(path).unwrap();
        assert_eq!(console.device_id(), device_id);
        let detail = console.ledger_detail(ledger.ledger_id).unwrap();
        assert_eq!(detail.summary.name, "Shared dinner");
        assert_eq!(detail.emoji_fingerprint.split(' ').count(), 6);
        assert!(root.join("unbill.sqlite3").is_file());
        assert_eq!(
            root.join("unbill.lock").exists(),
            !cfg!(any(
                target_os = "android",
                all(target_os = "ios", not(target_abi = "macabi"))
            ))
        );
        assert!(!root.join("device_key.bin").exists());
    }
}
