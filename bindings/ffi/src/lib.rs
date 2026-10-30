//! UniFFI adapters for the native `unbill` Console interface.
//!
//! All operations and projections are implemented by `unbill`. This crate only
//! converts boundary values and forwards callbacks and method calls.

use std::sync::Arc;

uniffi::setup_scaffolding!();

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ConsoleLegalSoftware {
    pub name: String,
    pub versions: Vec<String>,
    pub license_expressions: Vec<String>,
}

impl From<unbill::ConsoleLegalSoftware> for ConsoleLegalSoftware {
    fn from(value: unbill::ConsoleLegalSoftware) -> Self {
        let unbill::ConsoleLegalSoftware {
            name,
            versions,
            license_expressions,
        } = value;
        Self {
            name,
            versions,
            license_expressions,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ConsoleLegalDocument {
    pub title: String,
    pub text: String,
}

impl From<unbill::ConsoleLegalDocument> for ConsoleLegalDocument {
    fn from(value: unbill::ConsoleLegalDocument) -> Self {
        let unbill::ConsoleLegalDocument { title, text } = value;
        Self { title, text }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ConsoleLegalPackage {
    pub package_id: String,
    pub name: String,
    pub version: String,
    pub license_expression: String,
    pub repository: Option<String>,
    pub source: Option<String>,
    pub documents: Vec<ConsoleLegalDocument>,
}

impl From<unbill::ConsoleLegalPackage> for ConsoleLegalPackage {
    fn from(value: unbill::ConsoleLegalPackage) -> Self {
        let unbill::ConsoleLegalPackage {
            package_id,
            name,
            version,
            license_expression,
            repository,
            source,
            documents,
        } = value;
        Self {
            package_id,
            name,
            version,
            license_expression,
            repository,
            source,
            documents: documents.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ConsoleBuildInfo {
    pub version: String,
    pub built_at_utc: String,
}

impl From<unbill::ConsoleBuildInfo> for ConsoleBuildInfo {
    fn from(value: unbill::ConsoleBuildInfo) -> Self {
        let unbill::ConsoleBuildInfo {
            version,
            built_at_utc,
        } = value;
        Self {
            version,
            built_at_utc,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ConsoleLedgerSummary {
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

impl From<unbill::ConsoleLedgerSummary> for ConsoleLedgerSummary {
    fn from(value: unbill::ConsoleLedgerSummary) -> Self {
        let unbill::ConsoleLedgerSummary {
            archived,
            ledger_id,
            name,
            currency,
            created_at_ms,
            updated_at_ms,
            user_count,
            user_names,
            latest_bill_at_ms,
        } = value;
        Self {
            archived,
            ledger_id,
            name,
            currency,
            created_at_ms,
            updated_at_ms,
            user_count,
            user_names,
            latest_bill_at_ms,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ConsoleUser {
    pub user_id: String,
    pub display_name: String,
    pub archived: bool,
    pub added_at_ms: i64,
}

impl From<unbill::ConsoleUser> for ConsoleUser {
    fn from(value: unbill::ConsoleUser) -> Self {
        let unbill::ConsoleUser {
            user_id,
            display_name,
            archived,
            added_at_ms,
        } = value;
        Self {
            user_id,
            display_name,
            archived,
            added_at_ms,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ConsoleShare {
    pub user_id: String,
    pub shares: u32,
    pub display_name: String,
}

impl From<unbill::ConsoleShare> for ConsoleShare {
    fn from(value: unbill::ConsoleShare) -> Self {
        let unbill::ConsoleShare {
            user_id,
            shares,
            display_name,
        } = value;
        Self {
            user_id,
            shares,
            display_name,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ConsoleBill {
    pub id: String,
    pub amount_cents: i64,
    pub description: String,
    pub created_at_ms: i64,
    pub payers: Vec<ConsoleShare>,
    pub payees: Vec<ConsoleShare>,
    pub prev: Vec<String>,
}

impl From<unbill::ConsoleBill> for ConsoleBill {
    fn from(value: unbill::ConsoleBill) -> Self {
        let unbill::ConsoleBill {
            id,
            amount_cents,
            description,
            created_at_ms,
            payers,
            payees,
            prev,
        } = value;
        Self {
            id,
            amount_cents,
            description,
            created_at_ms,
            payers: payers.into_iter().map(Into::into).collect(),
            payees: payees.into_iter().map(Into::into).collect(),
            prev,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ConsoleUserAmount {
    pub user_id: String,
    pub amount_cents: i64,
}

impl From<unbill::ConsoleUserAmount> for ConsoleUserAmount {
    fn from(value: unbill::ConsoleUserAmount) -> Self {
        let unbill::ConsoleUserAmount {
            user_id,
            amount_cents,
        } = value;
        Self {
            user_id,
            amount_cents,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ConsoleBillSplit {
    pub payer_amounts: Vec<ConsoleUserAmount>,
    pub payee_amounts: Vec<ConsoleUserAmount>,
}

impl From<unbill::ConsoleBillSplit> for ConsoleBillSplit {
    fn from(value: unbill::ConsoleBillSplit) -> Self {
        let unbill::ConsoleBillSplit {
            payer_amounts,
            payee_amounts,
        } = value;
        Self {
            payer_amounts: payer_amounts.into_iter().map(Into::into).collect(),
            payee_amounts: payee_amounts.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ConsoleBillDraft {
    pub bill_id: String,
    pub prev_bill_ids: Vec<String>,
}

impl From<unbill::ConsoleBillDraft> for ConsoleBillDraft {
    fn from(value: unbill::ConsoleBillDraft) -> Self {
        let unbill::ConsoleBillDraft {
            bill_id,
            prev_bill_ids,
        } = value;
        Self {
            bill_id,
            prev_bill_ids,
        }
    }
}

impl From<ConsoleBillDraft> for unbill::ConsoleBillDraft {
    fn from(value: ConsoleBillDraft) -> Self {
        let ConsoleBillDraft {
            bill_id,
            prev_bill_ids,
        } = value;
        Self {
            bill_id,
            prev_bill_ids,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ConsoleTransaction {
    pub from_name: String,
    pub to_name: String,
    pub amount_cents: i64,
}

impl From<unbill::ConsoleTransaction> for ConsoleTransaction {
    fn from(value: unbill::ConsoleTransaction) -> Self {
        let unbill::ConsoleTransaction {
            from_name,
            to_name,
            amount_cents,
        } = value;
        Self {
            from_name,
            to_name,
            amount_cents,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ConsoleConflictGroup {
    pub conflicting: Vec<ConsoleBill>,
    pub ancestors: Vec<ConsoleBill>,
}

impl From<unbill::ConsoleConflictGroup> for ConsoleConflictGroup {
    fn from(value: unbill::ConsoleConflictGroup) -> Self {
        let unbill::ConsoleConflictGroup {
            conflicting,
            ancestors,
        } = value;
        Self {
            conflicting: conflicting.into_iter().map(Into::into).collect(),
            ancestors: ancestors.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ConsoleSyncDevice {
    pub node_id: String,
    pub label: String,
    pub ledger_names: Vec<String>,
}

impl From<unbill::ConsoleSyncDevice> for ConsoleSyncDevice {
    fn from(value: unbill::ConsoleSyncDevice) -> Self {
        let unbill::ConsoleSyncDevice {
            node_id,
            label,
            ledger_names,
        } = value;
        Self {
            node_id,
            label,
            ledger_names,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ConsoleBootstrap {
    pub device_id: String,
    pub ledgers: Vec<ConsoleLedgerSummary>,
    pub all_users: Vec<ConsoleUser>,
    pub devices: Vec<ConsoleSyncDevice>,
}

impl From<unbill::ConsoleBootstrap> for ConsoleBootstrap {
    fn from(value: unbill::ConsoleBootstrap) -> Self {
        let unbill::ConsoleBootstrap {
            device_id,
            ledgers,
            all_users,
            devices,
        } = value;
        Self {
            device_id,
            ledgers: ledgers.into_iter().map(Into::into).collect(),
            all_users: all_users.into_iter().map(Into::into).collect(),
            devices: devices.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ConsoleLedgerDetail {
    pub emoji_fingerprint: String,

    pub summary: ConsoleLedgerSummary,
    pub users: Vec<ConsoleUser>,
    pub devices: Vec<ConsoleSyncDevice>,
    pub bills: Vec<ConsoleBill>,
    pub conflicts: Vec<ConsoleConflictGroup>,
    pub settlement: Vec<ConsoleTransaction>,
}

impl From<unbill::ConsoleLedgerDetail> for ConsoleLedgerDetail {
    fn from(value: unbill::ConsoleLedgerDetail) -> Self {
        let unbill::ConsoleLedgerDetail {
            emoji_fingerprint,
            summary,
            users,
            devices,
            bills,
            conflicts,
            settlement,
        } = value;
        Self {
            emoji_fingerprint,
            summary: summary.into(),
            users: users.into_iter().map(Into::into).collect(),
            devices: devices.into_iter().map(Into::into).collect(),
            bills: bills.into_iter().map(Into::into).collect(),
            conflicts: conflicts.into_iter().map(Into::into).collect(),
            settlement: settlement.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ConsoleShareInput {
    pub user_id: String,
    pub shares: u32,
}

impl From<unbill::ConsoleShareInput> for ConsoleShareInput {
    fn from(value: unbill::ConsoleShareInput) -> Self {
        let unbill::ConsoleShareInput { user_id, shares } = value;
        Self { user_id, shares }
    }
}

impl From<ConsoleShareInput> for unbill::ConsoleShareInput {
    fn from(value: ConsoleShareInput) -> Self {
        let ConsoleShareInput { user_id, shares } = value;
        Self { user_id, shares }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ConsoleBlePeer {
    pub device_id: String,
    pub phase: ConsoleBlePeerPhase,
    pub consecutive_failures: u32,
    pub connect_path: Option<ConsoleBleConnectPath>,
    pub verified_node_id: Option<String>,
}

impl From<unbill::ConsoleBlePeer> for ConsoleBlePeer {
    fn from(value: unbill::ConsoleBlePeer) -> Self {
        let unbill::ConsoleBlePeer {
            device_id,
            phase,
            consecutive_failures,
            connect_path,
            verified_node_id,
        } = value;
        Self {
            device_id,
            phase: phase.into(),
            consecutive_failures,
            connect_path: connect_path.map(Into::into),
            verified_node_id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ConsoleBleSnapshot {
    pub state: ConsoleBleState,
    pub peers: Vec<ConsoleBlePeer>,
    pub discovered_node_ids: Vec<String>,
}

impl From<unbill::ConsoleBleSnapshot> for ConsoleBleSnapshot {
    fn from(value: unbill::ConsoleBleSnapshot) -> Self {
        let unbill::ConsoleBleSnapshot {
            state,
            peers,
            discovered_node_ids,
        } = value;
        Self {
            state: state.into(),
            peers: peers.into_iter().map(Into::into).collect(),
            discovered_node_ids,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum ConsoleBleState {
    Disabled,
    Active,
    Unavailable { reason: String },
    Closed,
}

impl From<unbill::ConsoleBleState> for ConsoleBleState {
    fn from(value: unbill::ConsoleBleState) -> Self {
        match value {
            unbill::ConsoleBleState::Disabled => Self::Disabled,
            unbill::ConsoleBleState::Active => Self::Active,
            unbill::ConsoleBleState::Unavailable { reason } => Self::Unavailable { reason },
            unbill::ConsoleBleState::Closed => Self::Closed,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum ConsoleBlePeerPhase {
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

impl From<unbill::ConsoleBlePeerPhase> for ConsoleBlePeerPhase {
    fn from(value: unbill::ConsoleBlePeerPhase) -> Self {
        match value {
            unbill::ConsoleBlePeerPhase::Discovered => Self::Discovered,
            unbill::ConsoleBlePeerPhase::PendingDial => Self::PendingDial,
            unbill::ConsoleBlePeerPhase::Connecting => Self::Connecting,
            unbill::ConsoleBlePeerPhase::Handshaking => Self::Handshaking,
            unbill::ConsoleBlePeerPhase::Connected => Self::Connected,
            unbill::ConsoleBlePeerPhase::Draining => Self::Draining,
            unbill::ConsoleBlePeerPhase::Reconnecting => Self::Reconnecting,
            unbill::ConsoleBlePeerPhase::Restoring => Self::Restoring,
            unbill::ConsoleBlePeerPhase::Unknown => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum ConsoleBleConnectPath {
    Gatt,
    L2cap,
}

impl From<unbill::ConsoleBleConnectPath> for ConsoleBleConnectPath {
    fn from(value: unbill::ConsoleBleConnectPath) -> Self {
        match value {
            unbill::ConsoleBleConnectPath::Gatt => Self::Gatt,
            unbill::ConsoleBleConnectPath::L2cap => Self::L2cap,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum ConsoleServiceEvent {
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

impl From<unbill::ConsoleServiceEvent> for ConsoleServiceEvent {
    fn from(value: unbill::ConsoleServiceEvent) -> Self {
        match value {
            unbill::ConsoleServiceEvent::LedgerUpdated { ledger_id } => {
                Self::LedgerUpdated { ledger_id }
            }
            unbill::ConsoleServiceEvent::PeerConnected { ledger_id, peer } => {
                Self::PeerConnected { ledger_id, peer }
            }
            unbill::ConsoleServiceEvent::PeerDisconnected { ledger_id, peer } => {
                Self::PeerDisconnected { ledger_id, peer }
            }
            unbill::ConsoleServiceEvent::SyncError {
                ledger_id,
                peer,
                error,
            } => Self::SyncError {
                ledger_id,
                peer,
                error,
            },
            unbill::ConsoleServiceEvent::ResyncNeeded => Self::ResyncNeeded,
        }
    }
}

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum ConsoleError {
    #[error("data directory is already in use: {path}")]
    DataDirectoryInUse { path: String },
    #[error("{0}")]
    Message(String),
}

impl From<unbill::ConsoleError> for ConsoleError {
    fn from(error: unbill::ConsoleError) -> Self {
        match error {
            unbill::ConsoleError::DataDirectoryInUse { path } => Self::DataDirectoryInUse { path },
            unbill::ConsoleError::Message(message) => Self::Message(message),
        }
    }
}

#[uniffi::export(callback_interface)]
pub trait ConsoleObserver: Send + Sync {
    /// Called on a worker thread; forward work before calling synchronous methods.
    fn on_event(&self, event: ConsoleServiceEvent);
}

struct ObserverAdapter(Box<dyn ConsoleObserver>);

impl unbill::ConsoleObserver for ObserverAdapter {
    fn on_event(&self, event: unbill::ConsoleServiceEvent) {
        self.0.on_event(event.into());
    }
}

#[uniffi::export(callback_interface)]
pub trait ConsoleBleObserver: Send + Sync {
    /// Called on a worker thread; dispatch UI changes to the main thread.
    fn on_snapshot(&self, snapshot: ConsoleBleSnapshot);
}

struct BleObserverAdapter(Box<dyn ConsoleBleObserver>);

impl unbill::ConsoleBleObserver for BleObserverAdapter {
    fn on_snapshot(&self, snapshot: unbill::ConsoleBleSnapshot) {
        self.0.on_snapshot(snapshot.into());
    }
}

/// Handle to an active event subscription; dropping it stops delivery.
#[derive(uniffi::Object)]
pub struct ConsoleSubscription {
    inner: Arc<unbill::ConsoleSubscription>,
}

#[uniffi::export]
impl ConsoleSubscription {
    pub fn cancel(&self) {
        self.inner.cancel();
    }
}

// sirno:witness:unbill-ffi:begin
#[derive(uniffi::Object)]
pub struct Console {
    inner: Arc<unbill::Console>,
}

#[uniffi::export]
impl Console {
    /// Run a device and console in this process, owning the SQLite data directory.
    #[uniffi::constructor]
    pub fn open_in_process(dir: String) -> Result<Arc<Self>, ConsoleError> {
        unbill::Console::open_in_process(dir)
            .map(|inner| Arc::new(Self { inner }))
            .map_err(Into::into)
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
    pub fn open_rpc(dir: String) -> Result<Arc<Self>, ConsoleError> {
        unbill::Console::open_rpc(dir)
            .map(|inner| Arc::new(Self { inner }))
            .map_err(Into::into)
    }
}

#[uniffi::export]
impl Console {
    pub fn service_build_info(&self) -> Result<ConsoleBuildInfo, ConsoleError> {
        self.inner
            .service_build_info()
            .map(Into::into)
            .map_err(Into::into)
    }

    pub fn device_id(&self) -> String {
        self.inner.device_id()
    }

    pub fn bootstrap(&self) -> Result<ConsoleBootstrap, ConsoleError> {
        self.inner.bootstrap().map(Into::into).map_err(Into::into)
    }

    pub fn ledger_detail(&self, ledger_id: String) -> Result<ConsoleLedgerDetail, ConsoleError> {
        self.inner
            .ledger_detail(ledger_id)
            .map(Into::into)
            .map_err(Into::into)
    }

    pub fn create_ledger(
        &self,
        name: String,
        currency: String,
    ) -> Result<ConsoleLedgerSummary, ConsoleError> {
        self.inner
            .create_ledger(name, currency)
            .map(Into::into)
            .map_err(Into::into)
    }

    pub fn create_user(
        &self,
        ledger_id: String,
        display_name: String,
    ) -> Result<ConsoleUser, ConsoleError> {
        self.inner
            .create_user(ledger_id, display_name)
            .map(Into::into)
            .map_err(Into::into)
    }

    pub fn set_ledger_archived(
        &self,
        ledger_id: String,
        archived: bool,
    ) -> Result<(), ConsoleError> {
        self.inner
            .set_ledger_archived(ledger_id, archived)
            .map_err(Into::into)
    }

    pub fn set_user_archived(
        &self,
        ledger_id: String,
        user_id: String,
        archived: bool,
    ) -> Result<(), ConsoleError> {
        self.inner
            .set_user_archived(ledger_id, user_id, archived)
            .map_err(Into::into)
    }

    pub fn rename_device(
        &self,
        ledger_id: String,
        node_id: String,
        name: String,
    ) -> Result<(), ConsoleError> {
        self.inner
            .rename_device(ledger_id, node_id, name)
            .map_err(Into::into)
    }

    pub fn add_user(
        &self,
        ledger_id: String,
        user_id: String,
    ) -> Result<ConsoleUser, ConsoleError> {
        self.inner
            .add_user(ledger_id, user_id)
            .map(Into::into)
            .map_err(Into::into)
    }

    pub fn preview_bill_split(
        &self,
        amount_cents: i64,
        payers: Vec<ConsoleShareInput>,
        payees: Vec<ConsoleShareInput>,
        bill_id: String,
    ) -> Result<ConsoleBillSplit, ConsoleError> {
        self.inner
            .preview_bill_split(
                amount_cents,
                payers.into_iter().map(Into::into).collect(),
                payees.into_iter().map(Into::into).collect(),
                bill_id,
            )
            .map(Into::into)
            .map_err(Into::into)
    }

    pub fn save_bill(
        &self,
        ledger_id: String,
        amount_cents: i64,
        description: String,
        payers: Vec<ConsoleShareInput>,
        payees: Vec<ConsoleShareInput>,
        draft: ConsoleBillDraft,
    ) -> Result<String, ConsoleError> {
        self.inner
            .save_bill(
                ledger_id,
                amount_cents,
                description,
                payers.into_iter().map(Into::into).collect(),
                payees.into_iter().map(Into::into).collect(),
                draft.into(),
            )
            .map_err(Into::into)
    }

    pub fn resolve_conflict(
        &self,
        ledger_id: String,
        selected_bill_id: String,
        conflicting_bill_ids: Vec<String>,
    ) -> Result<String, ConsoleError> {
        self.inner
            .resolve_conflict(ledger_id, selected_bill_id, conflicting_bill_ids)
            .map_err(Into::into)
    }

    pub fn create_invitation(&self, ledger_id: String) -> Result<String, ConsoleError> {
        self.inner.create_invitation(ledger_id).map_err(Into::into)
    }

    pub fn join_ledger(&self, url: String, label: String) -> Result<(), ConsoleError> {
        self.inner.join_ledger(url, label).map_err(Into::into)
    }

    pub async fn sync_once(&self, peer_node_id: String) -> Result<(), ConsoleError> {
        self.inner.sync_once(peer_node_id).await.map_err(Into::into)
    }

    pub async fn observe_ble(
        &self,
        observer: Box<dyn ConsoleBleObserver>,
    ) -> Result<Arc<ConsoleSubscription>, ConsoleError> {
        self.inner
            .observe_ble(Box::new(BleObserverAdapter(observer)))
            .await
            .map(|inner| Arc::new(ConsoleSubscription { inner }))
            .map_err(Into::into)
    }

    pub fn observe(&self, observer: Box<dyn ConsoleObserver>) -> Arc<ConsoleSubscription> {
        Arc::new(ConsoleSubscription {
            inner: self.inner.observe(Box::new(ObserverAdapter(observer))),
        })
    }
}
// sirno:witness:unbill-ffi:end

#[uniffi::export]
pub fn license_software_summaries() -> Vec<ConsoleLegalSoftware> {
    unbill::license_software_summaries()
        .into_iter()
        .map(Into::into)
        .collect()
}

#[uniffi::export]
pub fn license_software_details(name: String) -> Vec<ConsoleLegalPackage> {
    unbill::license_software_details(name)
        .into_iter()
        .map(Into::into)
        .collect()
}

#[uniffi::export]
pub fn supported_currency_codes() -> Vec<String> {
    unbill::supported_currency_codes()
}

#[uniffi::export]
pub fn default_data_directory() -> Result<String, ConsoleError> {
    unbill::default_data_directory().map_err(Into::into)
}

#[uniffi::export]
pub fn client_build_info() -> ConsoleBuildInfo {
    unbill::client_build_info().into()
}

#[uniffi::export]
pub fn new_bill_id() -> String {
    unbill::new_bill_id()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::time::Duration;

    #[test]
    fn foreign_operations_use_the_native_console_and_preserve_nested_values() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().to_str().unwrap().to_owned();
        let console = Console::open_in_process(path.clone()).unwrap();
        #[cfg(not(any(
            target_os = "android",
            all(target_os = "ios", not(target_abi = "macabi"))
        )))]
        {
            assert!(matches!(
                unbill::Console::open_in_process(path.clone()),
                Err(unbill::ConsoleError::DataDirectoryInUse { .. })
            ));
            assert!(matches!(
                Console::open_in_process(path.clone()),
                Err(ConsoleError::DataDirectoryInUse { .. })
            ));
        }
        let ledger = console
            .create_ledger("Shared".into(), "USD".into())
            .unwrap();
        let user = console
            .create_user(ledger.ledger_id.clone(), "Alice".into())
            .unwrap();
        let shares = vec![ConsoleShareInput {
            user_id: user.user_id,
            shares: 1,
        }];
        let bill_id = new_bill_id();
        console
            .save_bill(
                ledger.ledger_id.clone(),
                1234,
                "Dinner".into(),
                shares.clone(),
                shares,
                ConsoleBillDraft {
                    bill_id: bill_id.clone(),
                    prev_bill_ids: vec![],
                },
            )
            .unwrap();
        let detail = console.ledger_detail(ledger.ledger_id.clone()).unwrap();
        let native = console
            .inner
            .ledger_detail(ledger.ledger_id.clone())
            .unwrap();
        assert_eq!(detail, native.into());
        assert_eq!(detail.bills[0].id, bill_id);
        assert_eq!(detail.bills[0].payers[0].display_name, "Alice");
        assert_eq!(
            console.bootstrap().unwrap(),
            console.inner.bootstrap().unwrap().into()
        );
        assert!(matches!(
            console.ledger_detail("invalid".into()),
            Err(ConsoleError::Message(_))
        ));
        drop(console);

        // Rust consumers can reopen the exact data created through the adapter.
        let native = unbill::Console::open_in_process(path).unwrap();
        assert_eq!(
            native.ledger_detail(ledger.ledger_id).unwrap().bills[0].id,
            bill_id
        );
    }

    #[test]
    fn callbacks_forward_native_events_and_release_on_cancel() {
        struct Observer(mpsc::Sender<ConsoleServiceEvent>);
        impl ConsoleObserver for Observer {
            fn on_event(&self, event: ConsoleServiceEvent) {
                let _ = self.0.send(event);
            }
        }
        let dir = tempfile::tempdir().unwrap();
        let console = Console::open_in_process(dir.path().to_str().unwrap().to_owned()).unwrap();
        let ledger = console
            .create_ledger("Events".into(), "USD".into())
            .unwrap();
        let (tx, rx) = mpsc::channel();
        let subscription = console.observe(Box::new(Observer(tx)));
        console
            .create_user(ledger.ledger_id.clone(), "Alice".into())
            .unwrap();
        assert_eq!(
            rx.recv_timeout(Duration::from_secs(5)).unwrap(),
            ConsoleServiceEvent::LedgerUpdated {
                ledger_id: ledger.ledger_id
            },
        );
        subscription.cancel();
        loop {
            match rx.recv_timeout(Duration::from_secs(5)) {
                Ok(_) => continue,
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
                Err(error) => panic!("cancelled observer was not released: {error}"),
            }
        }
    }

    #[test]
    fn ble_callbacks_forward_the_initial_snapshot_and_release_on_drop() {
        struct Observer(mpsc::Sender<ConsoleBleSnapshot>);
        impl ConsoleBleObserver for Observer {
            fn on_snapshot(&self, snapshot: ConsoleBleSnapshot) {
                let _ = self.0.send(snapshot);
            }
        }
        let dir = tempfile::tempdir().unwrap();
        let console = Console::open_in_process(dir.path().to_str().unwrap().to_owned()).unwrap();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let (tx, rx) = mpsc::channel();
        let subscription = runtime
            .block_on(console.observe_ble(Box::new(Observer(tx))))
            .unwrap();
        let snapshot = rx.recv_timeout(Duration::from_secs(5)).unwrap();
        assert!(matches!(snapshot.state, ConsoleBleState::Disabled));
        assert!(snapshot.peers.is_empty());
        drop(subscription);
        assert!(matches!(
            rx.recv_timeout(Duration::from_secs(5)),
            Err(mpsc::RecvTimeoutError::Disconnected),
        ));
    }
}
