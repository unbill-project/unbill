use thiserror::Error;
use unbill_model_verified::ledger::check as vc;

// ---------------------------------------------------------------------------
// Per-operation error types (verified check + reconcile)
// ---------------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum AddUserOpError {
    #[error(transparent)]
    Check(#[from] vc::AddUserError),
    #[error("reconcile error: {0}")]
    Reconcile(String),
}

#[derive(Debug, Error)]
pub enum AddDeviceOpError {
    #[error(transparent)]
    Check(#[from] vc::AddDeviceError),
    #[error("reconcile error: {0}")]
    Reconcile(String),
}

#[derive(Debug, Error)]
pub enum AddBillOpError {
    #[error(transparent)]
    Check(#[from] vc::AddBillError),
    #[error("reconcile error: {0}")]
    Reconcile(String),
}

// ---------------------------------------------------------------------------
// Top-level error
// ---------------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum UnbillError {
    #[error(transparent)]
    AddUser(#[from] AddUserOpError),

    #[error(transparent)]
    AddDevice(#[from] AddDeviceOpError),

    #[error(transparent)]
    AddBill(#[from] AddBillOpError),

    #[error("ledger not found: {0}")]
    LedgerNotFound(String),

    #[error("user not found: {0}")]
    UserNotFound(String),

    #[error("device not found: {0}")]
    DeviceNotFound(String),

    #[error("invitation invalid or expired")]
    InvalidInvitation,

    #[error("not authorized")]
    NotAuthorized,

    #[error("validation error: {0}")]
    Validation(String),

    #[error("storage error: {0}")]
    Storage(#[from] StorageError),

    #[error("automerge error: {0}")]
    Automerge(String),

    #[error("reconcile error: {0}")]
    Reconcile(String),

    #[error("network error: {0}")]
    Network(String),

    #[error("invalid url: {0}")]
    InvalidUrl(String),

    #[error("no network feature enabled")]
    NoNetworkFeature,

    #[error("invalid ID {value:?}: {source}")]
    ParseId {
        value: String,
        source: ulid::DecodeError,
    },

    #[error("configuration error: {0}")]
    Config(String),
}

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("serialization error: {0}")]
    Serialization(String),

    #[error("unauthorized")]
    Unauthorized,

    #[error("http status {0}: {1}")]
    HttpStatus(u16, String),

    #[error("store server channel closed")]
    ChannelClosed,
}

pub type Result<T> = std::result::Result<T, UnbillError>;
