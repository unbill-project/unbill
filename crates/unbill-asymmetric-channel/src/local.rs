// LocalAsymChannel: in-process AsymChannel backed by unbill-service.

use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::broadcast;
use unbill_model::error::Result;
use unbill_model::{LedgerId, NodeId};
use unbill_service::ServiceEvent;
use unbill_service::{LedgerStore, UnbillService};

use crate::{AsymChannel, AsymChannelEvent};

pub struct LocalAsymChannel {
    service: Arc<UnbillService>,
    events: broadcast::Sender<AsymChannelEvent>,
}

impl LocalAsymChannel {
    pub async fn open<S: LedgerStore + Send + Sync + 'static>(store: Arc<S>) -> Result<Arc<Self>> {
        let service = UnbillService::open(store).await?;
        let (events, _) = broadcast::channel(256);

        // Forward ServiceEvents into the AsymChannelEvent broadcast.
        let mut svc_rx = service.subscribe();
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

        Ok(Arc::new(Self { service, events }))
    }

    pub fn service(&self) -> &Arc<UnbillService> {
        &self.service
    }

    pub async fn accept_loop(self: &Arc<Self>) -> Result<()> {
        self.service.accept_loop().await
    }
}

#[async_trait]
impl AsymChannel for LocalAsymChannel {
    async fn create_invitation(&self, ledger_id: LedgerId) -> Result<String> {
        self.service.create_invitation(ledger_id).await
    }

    async fn join_ledger(&self, url: String) -> Result<()> {
        self.service.join_ledger(&url, None).await
    }

    async fn trigger_peer_sync(&self, peer: NodeId) -> Result<()> {
        self.service.trigger_peer_sync(peer).await
    }

    async fn asym_sync(&self, ledger_id: LedgerId, bytes: Vec<u8>) -> Result<Option<Vec<u8>>> {
        self.service.asym_sync(ledger_id, bytes).await
    }

    fn subscribe_to_server(&self) -> broadcast::Receiver<AsymChannelEvent> {
        self.events.subscribe()
    }
}
