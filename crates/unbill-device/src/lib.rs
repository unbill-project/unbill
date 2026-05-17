// unbill-device: device-side implementation of the AsymChannel contract.
//
// Holds a local LedgerStore and provides invitation management, peer sync,
// and the Automerge sync server-side handler.  Does not know about the
// AsymChannel trait — LocalAsymChannel wraps this and wires up the trait.

use std::sync::Arc;

pub use unbill_event::ServiceEvent;
pub use unbill_storage::LedgerStore;

use automerge::sync::Message;
use tokio::sync::broadcast;
use unbill_model::error::{Result, UnbillError};
use unbill_model::{Invitation, InviteToken, LedgerId, NodeId, Timestamp};
use unbill_storage::{LedgerDoc, load_pending_invitations, save_pending_invitations};
use unbill_symmetric_channel::{JoinRequest, UnbillEndpoint};

// sirno:witness:unbill-device:begin
pub struct UnbillDevice {
    store: Arc<dyn LedgerStore>,
    device_id: NodeId,
    endpoint: UnbillEndpoint,
}

impl UnbillDevice {
    pub async fn open(store: Arc<dyn LedgerStore>) -> Result<Arc<Self>> {
        store.create_secret_key().await?;
        let device_id = store.get_device_id().await?;
        let key = store.get_secret_key().await?;
        let endpoint = UnbillEndpoint::bind(&key).await?;
        Ok(Arc::new(Self {
            store,
            device_id,
            endpoint,
        }))
    }

    pub fn store(&self) -> &Arc<dyn LedgerStore> {
        &self.store
    }

    pub fn device_id(&self) -> NodeId {
        self.device_id.clone()
    }

    pub fn subscribe(&self) -> broadcast::Receiver<ServiceEvent> {
        self.store.subscribe()
    }

    /// Generate a join invite URL for `ledger_id` and persist the pending invitation.
    pub async fn create_invitation(&self, ledger_id: LedgerId) -> Result<String> {
        let _ = self
            .store
            .load_ledger(&ledger_id.to_string())
            .await?
            .ok_or_else(|| UnbillError::LedgerNotFound(ledger_id.to_string()))?;
        let token = InviteToken::generate();
        let now = Timestamp::now();
        let invitation = Invitation {
            token: token.clone(),
            ledger_id,
            created_by_device: self.device_id.clone(),
            created_at: now,
            expires_at: Timestamp::from_millis(now.as_millis() + 24 * 3600 * 1000),
        };
        let mut map = load_pending_invitations(&*self.store).await?;
        map.insert(token.to_string(), invitation);
        save_pending_invitations(&*self.store, &map).await?;
        Ok(format!(
            "unbill://join/{}/{}/{}",
            ledger_id, self.device_id, token
        ))
    }

    /// Dial the host in `url` and join the ledger. `label` is an optional
    /// device-local nickname for the host stored after a successful join.
    pub async fn join_ledger(&self, url: &str, label: Option<String>) -> Result<()> {
        let (ledger_id, host, token) = parse_join_url(url)?;
        self.endpoint
            .join_ledger_inner(host, label, JoinRequest { token, ledger_id }, &self.store)
            .await
    }

    /// Collect all unique peer NodeIds across all ledgers, excluding this device.
    pub async fn collect_peers(&self) -> Result<Vec<NodeId>> {
        collect_peers_from_store(&*self.store, &self.device_id).await
    }

    /// Dial `peer` and run the full sync exchange for all shared ledgers.
    pub async fn trigger_peer_sync(&self, peer: NodeId) -> Result<()> {
        self.endpoint.sync_once_inner(peer, &self.store).await
    }

    /// Receive one Automerge sync message, persist any changes, and return the
    /// server's response (or `None` if it has nothing new to send).
    pub async fn asym_sync(&self, ledger_id: LedgerId, bytes: Vec<u8>) -> Result<Option<Vec<u8>>> {
        let id_str = ledger_id.to_string();
        let client_msg =
            Message::decode(&bytes).map_err(|e| UnbillError::Automerge(e.to_string()))?;
        let mut doc = self
            .store
            .load_ledger(&id_str)
            .await?
            .unwrap_or_else(LedgerDoc::empty);
        let mut sync_state = automerge::sync::State::new();
        let heads_before = doc.heads();
        doc.receive_sync_message(&mut sync_state, client_msg)
            .map_err(|e| UnbillError::Automerge(e.to_string()))?;
        if doc.heads() != heads_before {
            self.store.save_ledger(&id_str, &mut doc).await?;
        }
        Ok(doc
            .generate_sync_message(&mut sync_state)
            .map(|m| m.encode()))
    }

    /// Wait for the endpoint to be ready, print the readiness line, then accept
    /// incoming sync/join connections until an error occurs or the endpoint closes.
    pub async fn accept_loop(&self) -> Result<()> {
        self.endpoint.wait_for_ready().await;
        println!("listening on: {}", self.endpoint.node_id());
        self.endpoint
            .accept_loop_inner(Arc::clone(&self.store))
            .await
    }
}
// sirno:witness:unbill-device:end

async fn collect_peers_from_store(
    store: &dyn LedgerStore,
    self_id: &NodeId,
) -> Result<Vec<NodeId>> {
    let metas = store.list_ledgers().await?;
    let mut peers = Vec::new();
    for meta in metas {
        let id = meta.ledger_id.to_string();
        if let Some(doc) = store.load_ledger(&id).await? {
            if let Ok(devices) = doc.list_devices() {
                for device in devices {
                    if device.node_id != *self_id && !peers.contains(&device.node_id) {
                        peers.push(device.node_id);
                    }
                }
            }
        }
    }
    Ok(peers)
}

fn parse_join_url(url: &str) -> Result<(String, NodeId, String)> {
    let path = url
        .strip_prefix("unbill://join/")
        .ok_or_else(|| UnbillError::InvalidUrl(format!("invalid join URL: {url:?}")))?;
    let parts: Vec<&str> = path.splitn(3, '/').collect();
    if parts.len() != 3 {
        return Err(UnbillError::InvalidUrl(format!(
            "invalid join URL (expected ledger_id/host_node_id/token): {url:?}"
        )));
    }
    let ledger_id = parts[0].to_string();
    let host = parts[1]
        .parse::<NodeId>()
        .map_err(|e| UnbillError::InvalidUrl(format!("invalid host node ID in URL: {e}")))?;
    let token = parts[2].to_string();
    Ok((ledger_id, host, token))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use unbill_model::{Currency, LedgerId, NewDevice, NodeId, Timestamp};
    use unbill_storage::{LedgerDoc, LedgerStore};
    use unbill_store_memory::InMemoryStore;

    use super::collect_peers_from_store;

    fn usd() -> Currency {
        Currency::from_code("USD").unwrap()
    }

    #[tokio::test]
    async fn test_collect_peers_returns_unique_peers_excluding_self() {
        let store = Arc::new(InMemoryStore::default());
        let self_id = NodeId::from_seed(1);
        let peer_a = NodeId::from_seed(2);
        let peer_b = NodeId::from_seed(3);

        // Ledger 1: self + peer_a
        let mut doc1 =
            LedgerDoc::new(LedgerId::new(), "L1".to_string(), usd(), Timestamp::now()).unwrap();
        doc1.add_device(
            NewDevice {
                node_id: self_id.clone(),
            },
            Timestamp::now(),
        )
        .unwrap();
        doc1.add_device(
            NewDevice {
                node_id: peer_a.clone(),
            },
            Timestamp::now(),
        )
        .unwrap();
        let meta1 = unbill_model::LedgerMeta {
            ledger_id: doc1.get_ledger().unwrap().ledger_id,
            name: "L1".to_string(),
            currency: usd(),
            created_at: Timestamp::now(),
            updated_at: Timestamp::now(),
        };
        store.save_ledger_meta(&meta1).await.unwrap();
        store
            .save_ledger(&meta1.ledger_id.to_string(), &mut doc1)
            .await
            .unwrap();

        // Ledger 2: self + peer_a + peer_b
        let mut doc2 =
            LedgerDoc::new(LedgerId::new(), "L2".to_string(), usd(), Timestamp::now()).unwrap();
        doc2.add_device(
            NewDevice {
                node_id: self_id.clone(),
            },
            Timestamp::now(),
        )
        .unwrap();
        doc2.add_device(
            NewDevice {
                node_id: peer_a.clone(),
            },
            Timestamp::now(),
        )
        .unwrap();
        doc2.add_device(
            NewDevice {
                node_id: peer_b.clone(),
            },
            Timestamp::now(),
        )
        .unwrap();
        let meta2 = unbill_model::LedgerMeta {
            ledger_id: doc2.get_ledger().unwrap().ledger_id,
            name: "L2".to_string(),
            currency: usd(),
            created_at: Timestamp::now(),
            updated_at: Timestamp::now(),
        };
        store.save_ledger_meta(&meta2).await.unwrap();
        store
            .save_ledger(&meta2.ledger_id.to_string(), &mut doc2)
            .await
            .unwrap();

        let peers = collect_peers_from_store(&*store, &self_id).await.unwrap();

        assert_eq!(peers.len(), 2);
        assert!(peers.contains(&peer_a));
        assert!(peers.contains(&peer_b));
        assert!(!peers.contains(&self_id));
    }

    #[tokio::test]
    async fn test_collect_peers_empty_when_no_ledgers() {
        let store = Arc::new(InMemoryStore::default());
        let self_id = NodeId::from_seed(1);

        let peers = collect_peers_from_store(&*store, &self_id).await.unwrap();
        assert!(peers.is_empty());
    }
}
