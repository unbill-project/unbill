// In-process AsymChannel implementation backed directly by device storage.

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use tokio::sync::broadcast;
use unbill_event::ServiceEvent;
use unbill_model::error::{Result, UnbillError};
use unbill_model::{Invitation, InviteToken, LedgerId, NodeId, Timestamp};
use unbill_storage::{LedgerDoc, LedgerStore, load_pending_invitations, save_pending_invitations};
use unbill_symmetric_channel::{JoinRequest, UnbillEndpoint};

use crate::{AsymChannel, AsymChannelEvent};

pub struct LocalAsymChannel {
    store: Arc<dyn LedgerStore>,
    device_id: NodeId,
    endpoint: Mutex<Option<Arc<UnbillEndpoint>>>,
    svc_events: broadcast::Sender<ServiceEvent>,
    events: broadcast::Sender<AsymChannelEvent>,
}

impl LocalAsymChannel {
    pub async fn open(store: Arc<dyn LedgerStore>) -> Result<Arc<Self>> {
        store.create_secret_key().await?;
        let device_id = store.get_device_id().await?;
        let (svc_events, _) = broadcast::channel(256);
        let (events, _) = broadcast::channel(256);

        // Forward ServiceEvents into the AsymChannelEvent broadcast.
        let mut svc_rx = svc_events.subscribe();
        let tx = events.clone();
        tokio::spawn(async move {
            while let Ok(evt) = svc_rx.recv().await {
                let asym_evt = match evt {
                    ServiceEvent::LedgerUpdated { ledger_id } => LedgerId::from_string(&ledger_id)
                        .ok()
                        .map(|lid| AsymChannelEvent::LedgerUpdated { ledger_id: lid }),
                    _ => None,
                };
                if let Some(e) = asym_evt {
                    let _ = tx.send(e);
                }
            }
        });

        Ok(Arc::new(Self {
            store,
            device_id,
            endpoint: Mutex::new(None),
            svc_events,
            events,
        }))
    }

    pub async fn accept_loop(self: &Arc<Self>) -> Result<()> {
        let key = self.store.get_secret_key().await?;
        let ep = Arc::new(UnbillEndpoint::bind(&key).await?);
        ep.wait_for_ready().await;
        println!("listening on: {}", ep.node_id());
        *self.endpoint.lock().unwrap() = Some(Arc::clone(&ep));
        let result = ep
            .accept_loop_inner(Arc::clone(&self.store), self.svc_events.clone())
            .await;
        *self.endpoint.lock().unwrap() = None;
        ep.close().await;
        result
    }
}

#[async_trait]
impl AsymChannel for LocalAsymChannel {
    async fn create_invitation(&self, ledger_id: LedgerId) -> Result<String> {
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
        {
            let mut map = load_pending_invitations(&*self.store).await?;
            map.insert(token.to_string(), invitation);
            save_pending_invitations(&*self.store, &map).await?;
        }
        Ok(format!(
            "unbill://join/{}/{}/{}",
            ledger_id, self.device_id, token
        ))
    }

    async fn join_ledger(&self, url: String) -> Result<()> {
        let (ledger_id, host, token) = parse_join_url(&url)?;
        let request = JoinRequest { token, ledger_id };
        let ep = self.endpoint.lock().unwrap().clone();
        if let Some(ep) = ep {
            return ep
                .join_ledger_inner(host, None, request, &self.store, &self.svc_events)
                .await;
        }
        let key = self.store.get_secret_key().await?;
        let ep = UnbillEndpoint::bind(&key).await?;
        let result = ep
            .join_ledger_inner(host, None, request, &self.store, &self.svc_events)
            .await;
        ep.close().await;
        result
    }

    async fn trigger_peer_sync(&self, peer: NodeId) -> Result<()> {
        let ep = self.endpoint.lock().unwrap().clone();
        if let Some(ep) = ep {
            return ep
                .sync_once_inner(peer, &self.store, &self.svc_events)
                .await;
        }
        let key = self.store.get_secret_key().await?;
        let ep = UnbillEndpoint::bind(&key).await?;
        let result = ep
            .sync_once_inner(peer, &self.store, &self.svc_events)
            .await;
        ep.close().await;
        result
    }

    async fn asym_sync(&self, ledger_id: LedgerId, bytes: Vec<u8>) -> Result<Option<Vec<u8>>> {
        let id_str = ledger_id.to_string();
        let client_msg = automerge::sync::Message::decode(&bytes)
            .map_err(|e| UnbillError::Automerge(e.to_string()))?;
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

    fn subscribe_to_server(&self) -> broadcast::Receiver<AsymChannelEvent> {
        self.events.subscribe()
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
