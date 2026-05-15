// In-process AsymChannel implementation backed directly by UnbillService.

use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::broadcast;
use unbill_core::LedgerDoc;
use unbill_core::service::UnbillService;
use unbill_model::error::{Result, UnbillError};
use unbill_model::{LedgerId, NodeId};

use crate::{AsymChannel, AsymChannelEvent};

pub struct LocalAsymChannel {
    service: Arc<UnbillService>,
    events: broadcast::Sender<AsymChannelEvent>,
}

impl LocalAsymChannel {
    pub fn new(service: Arc<UnbillService>) -> Self {
        let (tx, _) = broadcast::channel(256);
        // Forward ServiceEvents into the AsymChannelEvent broadcast.
        let mut svc_rx = service.subscribe();
        let tx2 = tx.clone();
        tokio::spawn(async move {
            while let Ok(evt) = svc_rx.recv().await {
                use unbill_core::event::ServiceEvent;
                let device_evt = match evt {
                    ServiceEvent::LedgerUpdated { ledger_id } => LedgerId::from_string(&ledger_id)
                        .ok()
                        .map(|lid| AsymChannelEvent::LedgerUpdated { ledger_id: lid }),
                    _ => None,
                };
                if let Some(e) = device_evt {
                    let _ = tx2.send(e);
                }
            }
        });
        LocalAsymChannel {
            service,
            events: tx,
        }
    }
}

#[async_trait]
impl AsymChannel for LocalAsymChannel {
    async fn create_invitation(&self, ledger_id: LedgerId) -> Result<String> {
        self.service.create_invitation(ledger_id).await
    }

    async fn join_ledger(&self, url: String) -> Result<()> {
        self.service.join_ledger(&url, String::new()).await
    }

    async fn trigger_peer_sync(&self, peer: NodeId) -> Result<()> {
        self.service.sync_once(peer).await
    }

    async fn asym_sync(&self, ledger_id: LedgerId, bytes: Vec<u8>) -> Result<Option<Vec<u8>>> {
        let id_str = ledger_id.to_string();

        let client_msg = automerge::sync::Message::decode(&bytes)
            .map_err(|e| UnbillError::Automerge(e.to_string()))?;

        let mut doc = self
            .service
            .store()
            .load_ledger(&id_str)
            .await?
            .unwrap_or_else(LedgerDoc::empty);

        let mut sync_state = automerge::sync::State::new();
        doc.receive_sync_message(&mut sync_state, client_msg)
            .map_err(|e| UnbillError::Automerge(e.to_string()))?;

        if !doc.is_empty() {
            self.service.store().save_ledger(&id_str, &mut doc).await?;
        }

        Ok(doc
            .generate_sync_message(&mut sync_state)
            .map(|m| m.encode()))
    }

    fn subscribe_to_server(&self) -> broadcast::Receiver<AsymChannelEvent> {
        self.events.subscribe()
    }
}
