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
