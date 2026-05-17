use thiserror::Error;

#[derive(Debug, Error)]
pub enum UnbillError {
    #[error("ledger not found: {0}")]
    LedgerNotFound(String),

    #[error("bill not found: {0}")]
    BillNotFound(String),

    #[error("user {0} is not in this ledger")]
    UserNotInLedger(String),

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
