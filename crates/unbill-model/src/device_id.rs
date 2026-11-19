use autosurgeon::{Hydrate, Reconcile};

use crate::{NodeId, ProfileId};

/// A ledger device identity. A server profile shares its server's Iroh key
/// while remaining a distinct ledger member and bill author.
#[derive(
    Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, Reconcile, Hydrate,
)]
pub enum DeviceId {
    Iroh(NodeId),
    ServerProfile {
        node_id: NodeId,
        profile_id: ProfileId,
    },
}

impl DeviceId {
    /// The authenticated Iroh peer used to reach this device identity.
    pub fn node_id(&self) -> &NodeId {
        match self {
            Self::Iroh(node_id) | Self::ServerProfile { node_id, .. } => node_id,
        }
    }
}

impl From<NodeId> for DeviceId {
    fn from(node_id: NodeId) -> Self {
        Self::Iroh(node_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use automerge::AutoCommit;

    #[derive(Debug, PartialEq, Eq, Reconcile, Hydrate)]
    struct IdentityRecord {
        id: DeviceId,
    }

    #[test]
    fn tagged_device_ids_round_trip_through_automerge() {
        let node_id = NodeId::from_seed(7);
        let profile_id = ProfileId::from_u128(42);
        let identities = [
            DeviceId::Iroh(node_id.clone()),
            DeviceId::ServerProfile {
                node_id: node_id.clone(),
                profile_id,
            },
        ];

        assert_ne!(identities[0], identities[1]);
        assert_ne!(
            identities[1],
            DeviceId::ServerProfile {
                node_id: node_id.clone(),
                profile_id: ProfileId::from_u128(43),
            }
        );
        assert_eq!(identities[0].node_id(), &node_id);
        assert_eq!(identities[1].node_id(), &node_id);

        for id in identities {
            let mut doc = AutoCommit::new();
            autosurgeon::reconcile(&mut doc, &IdentityRecord { id: id.clone() }).unwrap();
            let loaded = AutoCommit::load(&doc.save()).unwrap();
            let recovered: IdentityRecord = autosurgeon::hydrate(&loaded).unwrap();
            assert_eq!(recovered.id, id);
        }
    }

    #[test]
    fn tagged_device_ids_round_trip_through_json() {
        let node_id = NodeId::from_seed(7);
        for id in [
            DeviceId::Iroh(node_id.clone()),
            DeviceId::ServerProfile {
                node_id,
                profile_id: ProfileId::from_u128(42),
            },
        ] {
            let json = serde_json::to_string(&id).unwrap();
            let tag = match &id {
                DeviceId::Iroh(_) => "Iroh",
                DeviceId::ServerProfile { .. } => "ServerProfile",
            };
            assert!(json.contains(tag));
            let recovered: DeviceId = serde_json::from_str(&json).unwrap();
            assert_eq!(recovered, id);
        }
    }
}
