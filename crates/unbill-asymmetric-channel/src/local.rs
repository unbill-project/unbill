// LocalAsymChannel: in-process AsymChannel backed by unbill-device.

use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::broadcast;
use unbill_device::ServiceEvent;
use unbill_device::{LedgerStore, UnbillDevice};
use unbill_model::error::Result;
use unbill_model::{LedgerId, LedgerMeta, NodeId, UnbillError};

use crate::{AsymChannel, AsymChannelEvent};

pub struct LocalAsymChannel {
    service: Arc<UnbillDevice>,
    events: broadcast::Sender<AsymChannelEvent>,
}

impl LocalAsymChannel {
    pub async fn open<S: LedgerStore + Send + Sync + 'static>(store: Arc<S>) -> Result<Arc<Self>> {
        let service = UnbillDevice::open(store).await?;
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

    pub fn service(&self) -> &Arc<UnbillDevice> {
        &self.service
    }

    pub async fn accept_loop(self: &Arc<Self>) -> Result<()> {
        self.service.accept_loop().await
    }
}

#[async_trait]
impl AsymChannel for LocalAsymChannel {
    fn device_id(&self) -> NodeId {
        self.service.device_id()
    }

    async fn create_invitation(&self, ledger_id: LedgerId) -> Result<String> {
        self.service.create_invitation(ledger_id).await
    }

    async fn join_ledger(&self, url: String, label: Option<String>) -> Result<()> {
        self.service.join_ledger(&url, label).await
    }

    async fn trigger_peer_sync(&self, peer: NodeId) -> Result<()> {
        self.service.trigger_peer_sync(peer).await
    }

    async fn asym_sync(&self, ledger_id: LedgerId, bytes: Vec<u8>) -> Result<Option<Vec<u8>>> {
        self.service.asym_sync(ledger_id, bytes).await
    }

    async fn list_ledgers(&self) -> Result<Vec<LedgerMeta>> {
        self.service
            .store()
            .list_ledgers()
            .await
            .map_err(UnbillError::Storage)
    }

    async fn save_ledger_meta(&self, meta: &LedgerMeta) -> Result<()> {
        self.service
            .store()
            .save_ledger_meta(meta)
            .await
            .map_err(UnbillError::Storage)
    }

    async fn load_device_meta(&self, key: &str) -> Result<Option<Vec<u8>>> {
        self.service
            .store()
            .load_device_meta(key)
            .await
            .map_err(UnbillError::Storage)
    }

    async fn save_device_meta(&self, key: &str, bytes: Vec<u8>) -> Result<()> {
        self.service
            .store()
            .save_device_meta(key, &bytes)
            .await
            .map_err(UnbillError::Storage)
    }

    fn subscribe_to_server(&self) -> broadcast::Receiver<AsymChannelEvent> {
        self.events.subscribe()
    }
}
