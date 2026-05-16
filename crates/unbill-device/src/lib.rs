// unbill-device: device-side implementation of the AsymChannel contract.
//
// Holds a local LedgerStore and provides invitation management, peer sync,
// and the Automerge sync server-side handler.  Does not know about the
// AsymChannel trait — LocalAsymChannel wraps this and wires up the trait.

use std::sync::{Arc, Mutex};

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
    endpoint: Mutex<Option<Arc<UnbillEndpoint>>>,
    events: broadcast::Sender<ServiceEvent>,
}

impl UnbillDevice {
    pub async fn open(store: Arc<dyn LedgerStore>) -> Result<Arc<Self>> {
        store.create_secret_key().await?;
        let device_id = store.get_device_id().await?;
        let (events, _) = broadcast::channel(256);
        Ok(Arc::new(Self {
            store,
            device_id,
            endpoint: Mutex::new(None),
            events,
        }))
    }

    pub fn store(&self) -> &Arc<dyn LedgerStore> {
        &self.store
    }

    pub fn device_id(&self) -> NodeId {
        self.device_id.clone()
    }

    pub fn subscribe(&self) -> broadcast::Receiver<ServiceEvent> {
        self.events.subscribe()
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
        let request = JoinRequest { token, ledger_id };
        let ep = self.endpoint.lock().unwrap().clone();
        if let Some(ep) = ep {
            return ep
                .join_ledger_inner(host, label, request, &self.store, &self.events)
                .await;
        }
        let key = self.store.get_secret_key().await?;
        let ep = UnbillEndpoint::bind(&key).await?;
        let result = ep
            .join_ledger_inner(host, label, request, &self.store, &self.events)
            .await;
        ep.close().await;
        result
    }

    /// Dial `peer` and run the full sync exchange for all shared ledgers.
    pub async fn trigger_peer_sync(&self, peer: NodeId) -> Result<()> {
        let ep = self.endpoint.lock().unwrap().clone();
        if let Some(ep) = ep {
            return ep.sync_once_inner(peer, &self.store, &self.events).await;
        }
        let key = self.store.get_secret_key().await?;
        let ep = UnbillEndpoint::bind(&key).await?;
        let result = ep.sync_once_inner(peer, &self.store, &self.events).await;
        ep.close().await;
        result
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
        doc.receive_sync_message(&mut sync_state, client_msg)
            .map_err(|e| UnbillError::Automerge(e.to_string()))?;
        if !doc.is_empty() {
            self.store.save_ledger(&id_str, &mut doc).await?;
        }
        Ok(doc
            .generate_sync_message(&mut sync_state)
            .map(|m| m.encode()))
    }

    /// Bind an endpoint and accept incoming sync/join connections until an
    /// error occurs or the endpoint is closed.
    pub async fn accept_loop(self: &Arc<Self>) -> Result<()> {
        let key = self.store.get_secret_key().await?;
        let ep = Arc::new(UnbillEndpoint::bind(&key).await?);
        ep.wait_for_ready().await;
        println!("listening on: {}", ep.node_id());
        *self.endpoint.lock().unwrap() = Some(Arc::clone(&ep));
        let result = ep
            .accept_loop_inner(Arc::clone(&self.store), self.events.clone())
            .await;
        *self.endpoint.lock().unwrap() = None;
        ep.close().await;
        result
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
