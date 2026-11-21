// LocalAsymChannel: in-process AsymChannel backed by unbill-device.

use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::Mutex;
use tokio::sync::broadcast;
use unbill_device::ServiceEvent;
use unbill_device::UnbillDevice;
use unbill_model::error::Result;
use unbill_model::{LedgerId, LedgerMeta, NodeId, UnbillError};
use unbill_storage::LedgerStore;

use crate::{AsymChannel, AsymChannelEvent};

// sirno:witness:asymmetric-channel:begin
pub struct LocalAsymChannel {
    service: Arc<UnbillDevice>,
    session: Arc<Mutex<Option<String>>>,
    build_info: unbill_build_info::BuildInfo,
    events: broadcast::Sender<AsymChannelEvent>,
}

impl LocalAsymChannel {
    pub async fn open<S: LedgerStore + Send + Sync + 'static>(
        store: Arc<S>,
        build_info: unbill_build_info::BuildInfo,
    ) -> Result<Arc<Self>> {
        let service = UnbillDevice::open(store).await?;
        let (events, _) = broadcast::channel(256);

        let mut ble_rx = service.subscribe_ble();
        let ble_tx = events.clone();
        tokio::spawn(async move {
            while ble_rx.changed().await.is_ok() {
                let _ = ble_tx.send(AsymChannelEvent::BleSnapshotChanged);
            }
        });

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

        Ok(Arc::new(Self {
            service,
            session: Arc::new(Mutex::new(None)),
            events,
            build_info,
        }))
    }

    pub fn service(&self) -> &Arc<UnbillDevice> {
        &self.service
    }

    pub async fn accept_loop(&self) -> Result<()> {
        self.service.accept_loop().await
    }

    async fn view(&self) -> Result<unbill_device::AccountView> {
        let token = self
            .session
            .lock()
            .await
            .clone()
            .ok_or(UnbillError::NotAuthorized)?;
        self.service.session_view(&token).await
    }
}

#[async_trait]
impl AsymChannel for LocalAsymChannel {
    async fn register(&self, name: &str, password: &str) -> Result<()> {
        self.service.register(name, password).await
    }
    async fn login(&self, name: &str, password: &str) -> Result<String> {
        let token = self.service.login(name, password).await?;
        if let Some(previous) = self.session.lock().await.replace(token.clone()) {
            self.service.logout(&previous).await?;
        }
        Ok(token)
    }
    async fn resume_session(&self, token: &str) -> Result<String> {
        let name = self.service.session_account(token).await?;
        *self.session.lock().await = Some(token.to_owned());
        Ok(name)
    }
    async fn current_account(&self) -> Result<String> {
        let token = self
            .session
            .lock()
            .await
            .clone()
            .ok_or(UnbillError::NotAuthorized)?;
        self.service.session_account(&token).await
    }
    async fn logout(&self) -> Result<()> {
        let token = self
            .session
            .lock()
            .await
            .take()
            .ok_or(UnbillError::NotAuthorized)?;
        self.service.logout(&token).await
    }
    async fn change_password(&self, old: &str, new: &str) -> Result<()> {
        let token = self
            .session
            .lock()
            .await
            .clone()
            .ok_or(UnbillError::NotAuthorized)?;
        self.service.change_password(&token, old, new).await?;
        *self.session.lock().await = None;
        Ok(())
    }
    async fn list_accounts(&self) -> Result<Vec<String>> {
        self.view().await?.list_accounts().await
    }
    async fn share_ledger(&self, id: LedgerId, name: &str) -> Result<()> {
        self.view().await?.share_ledger(id, name).await
    }
    fn device_id(&self) -> NodeId {
        self.service.device_id()
    }

    async fn build_info(&self) -> Result<unbill_build_info::BuildInfo> {
        Ok(self.build_info.clone())
    }

    async fn ble_snapshot(&self) -> Result<unbill_model::BleSnapshot> {
        Ok(self.service.ble_snapshot())
    }

    async fn create_invitation(&self, ledger_id: LedgerId) -> Result<String> {
        self.view().await?.create_invitation(ledger_id).await
    }

    async fn join_ledger(&self, url: String, label: unbill_model::DeviceLabel) -> Result<()> {
        self.view().await?.join_ledger(&url, label).await
    }

    async fn trigger_peer_sync(&self, peer: NodeId) -> Result<()> {
        self.view().await?;
        self.service.trigger_peer_sync(peer).await
    }

    async fn asym_sync(&self, ledger_id: LedgerId, bytes: Vec<u8>) -> Result<Option<Vec<u8>>> {
        self.view().await?.asym_sync(ledger_id, bytes).await
    }

    async fn list_ledgers(&self) -> Result<Vec<LedgerMeta>> {
        self.view().await?.list_ledgers().await
    }

    async fn set_ledger_archived(&self, ledger_id: LedgerId, archived: bool) -> Result<()> {
        self.view()
            .await?
            .set_ledger_archived(ledger_id, archived)
            .await
    }

    async fn save_ledger_meta(&self, meta: &LedgerMeta) -> Result<()> {
        self.view().await?.save_ledger_meta(meta).await
    }

    fn subscribe_to_server(&self) -> broadcast::Receiver<AsymChannelEvent> {
        let mut source = self.events.subscribe();
        let (filtered, receiver) = broadcast::channel(256);
        let service = Arc::clone(&self.service);
        let session = Arc::clone(&self.session);
        tokio::spawn(async move {
            loop {
                let event = match source.recv().await {
                    Ok(event) => event,
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(broadcast::error::RecvError::Closed) => break,
                };
                let Some(token) = session.lock().await.clone() else {
                    continue;
                };
                let Ok(view) = service.session_view(&token).await else {
                    continue;
                };
                let allowed = match &event {
                    AsymChannelEvent::LedgerUpdated { ledger_id } => view
                        .list_ledgers()
                        .await
                        .is_ok_and(|metas| metas.iter().any(|meta| meta.ledger_id == *ledger_id)),
                    AsymChannelEvent::BleSnapshotChanged => true,
                };
                if allowed && filtered.send(event).is_err() {
                    break;
                }
            }
        });
        receiver
    }
}
// sirno:witness:asymmetric-channel:end
