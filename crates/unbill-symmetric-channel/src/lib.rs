#[cfg(any(
    target_os = "macos",
    target_os = "ios",
    target_os = "linux",
    target_os = "android"
))]
mod ble;
mod endpoint;
mod join;
mod node_id_ext;
mod protocol;
mod sync;

pub use endpoint::{AcceptedBi, ConnHandle, ConnectedBi, UnbillEndpoint};
pub use join::{run_join_host, run_join_requester};
pub use node_id_ext::{EndpointIdExt, IrohSecretKeyExt, NodeIdExt, SecretKeyExt};
pub use protocol::{
    ALPN_JOIN, ALPN_SYNC, JoinError, JoinReply, JoinRequest, JoinResponse, read_msg, write_msg,
};
pub use sync::run_sync_session;
