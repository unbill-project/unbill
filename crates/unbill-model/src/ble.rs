use serde::{Deserialize, Serialize};

use crate::NodeId;

// sirno:witness:ble-transport:begin
/// Process-local Bluetooth observations; these never grant ledger access.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BleSnapshot {
    #[serde(flatten)]
    pub state: BleState,
    pub peers: Vec<BlePeer>,
    /// Known IDs matched to peers in this snapshot, not authenticated identities.
    #[serde(default)]
    pub discovered_node_ids: Vec<NodeId>,
}

impl BleSnapshot {
    /// Current peers only; discovery history without a peer never produces a badge.
    pub fn observed_node_ids(&self) -> Vec<NodeId> {
        if self.state != BleState::Active || self.peers.is_empty() {
            return Vec::new();
        }
        let mut nodes = self.discovered_node_ids.clone();
        nodes.extend(
            self.peers
                .iter()
                .filter_map(|peer| peer.verified_node_id.clone()),
        );
        nodes.sort_by_key(ToString::to_string);
        nodes.dedup();
        nodes
    }

    /// Display projection: observed identities, then verified connected identities.
    pub fn indicator_node_ids(&self) -> (Vec<String>, Vec<String>) {
        (
            self.observed_node_ids()
                .into_iter()
                .map(|id| id.to_string())
                .collect(),
            self.connected_node_ids()
                .into_iter()
                .map(|id| id.to_string())
                .collect(),
        )
    }

    /// Verified identities with an established BLE connection, in stable order.
    /// This does not imply that Iroh currently routes sync traffic over BLE.
    pub fn connected_node_ids(&self) -> Vec<NodeId> {
        if self.state != BleState::Active {
            return Vec::new();
        }
        let mut nodes: Vec<_> = self
            .peers
            .iter()
            .filter(|peer| peer.phase == BlePeerPhase::Connected)
            .filter_map(|peer| peer.verified_node_id.clone())
            .collect();
        nodes.sort_by_key(ToString::to_string);
        nodes.dedup();
        nodes
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn historical_discovery_without_current_peers_has_no_indicator() {
        let snapshot = BleSnapshot {
            state: BleState::Active,
            peers: Vec::new(),
            discovered_node_ids: vec![NodeId::from_seed(2)],
        };
        assert_eq!(snapshot.indicator_node_ids(), (vec![], vec![]));
    }

    #[test]
    fn discovery_hints_are_gray_until_verified_connected() {
        let id = NodeId::from_seed(2);
        let mut snapshot = BleSnapshot {
            state: BleState::Active,
            peers: vec![BlePeer {
                device_id: "radio".into(),
                phase: BlePeerPhase::Discovered,
                consecutive_failures: 0,
                connect_path: None,
                verified_node_id: None,
            }],
            discovered_node_ids: vec![id.clone(), id.clone()],
        };
        assert_eq!(
            snapshot.indicator_node_ids(),
            (vec![id.to_string()], vec![])
        );
        snapshot.peers[0] = BlePeer {
            device_id: "radio".into(),
            phase: BlePeerPhase::Connected,
            consecutive_failures: 0,
            connect_path: Some(BleConnectPath::Gatt),
            verified_node_id: Some(id.clone()),
        };
        assert_eq!(
            snapshot.indicator_node_ids(),
            (vec![id.to_string()], vec![id.to_string()])
        );
        snapshot.peers[0].phase = BlePeerPhase::Reconnecting;
        assert!(snapshot.connected_node_ids().is_empty());
        assert_eq!(snapshot.observed_node_ids(), vec![id]);
        snapshot.state = BleState::Closed;
        assert_eq!(snapshot.indicator_node_ids(), (vec![], vec![]));
    }

    #[test]
    fn old_snapshots_default_to_no_discovery_hints() {
        let snapshot: BleSnapshot =
            serde_json::from_str(r#"{"state":"active","peers":[]}"#).unwrap();
        assert!(snapshot.discovered_node_ids.is_empty());
    }

    #[test]
    fn connection_badges_require_active_verified_connected_peers() {
        let id = NodeId::from_seed(1);
        let peer = BlePeer {
            device_id: "radio-peer".into(),
            phase: BlePeerPhase::Connected,
            consecutive_failures: 0,
            connect_path: Some(BleConnectPath::Gatt),
            verified_node_id: Some(id.clone()),
        };
        let mut snapshot = BleSnapshot {
            state: BleState::Active,
            discovered_node_ids: Vec::new(),
            peers: vec![peer.clone(), peer],
        };
        assert_eq!(snapshot.connected_node_ids(), vec![id]);
        for phase in [
            BlePeerPhase::Discovered,
            BlePeerPhase::Connecting,
            BlePeerPhase::Handshaking,
            BlePeerPhase::Reconnecting,
            BlePeerPhase::Draining,
        ] {
            for peer in &mut snapshot.peers {
                peer.phase = phase;
            }
            assert!(snapshot.connected_node_ids().is_empty());
        }
        for peer in &mut snapshot.peers {
            peer.phase = BlePeerPhase::Connected;
            peer.verified_node_id = None;
        }
        assert!(snapshot.connected_node_ids().is_empty());
        for peer in &mut snapshot.peers {
            peer.verified_node_id = Some(NodeId::from_seed(1));
        }
        snapshot.state = BleState::Closed;
        assert!(snapshot.connected_node_ids().is_empty());
    }
}
