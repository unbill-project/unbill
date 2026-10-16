use serde::{Deserialize, Serialize};

use crate::NodeId;

// sirno:witness:ble-transport:begin
/// Process-local Bluetooth observations; these never grant ledger access.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BleSnapshot {
    #[serde(flatten)]
    pub state: BleState,
    pub peers: Vec<BlePeer>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum BleState {
    Disabled,
    Active,
    Unavailable { reason: String },
    Closed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlePeer {
    /// Platform Bluetooth identifier, not an authenticated Unbill identity.
    pub device_id: String,
    pub phase: BlePeerPhase,
    pub consecutive_failures: u32,
    pub connect_path: Option<BleConnectPath>,
    /// Set only after the transport has authenticated the endpoint identity.
    pub verified_node_id: Option<NodeId>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlePeerPhase {
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BleConnectPath {
    Gatt,
    L2cap,
}
// sirno:witness:ble-transport:end
