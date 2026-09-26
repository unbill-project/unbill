// StoreServer: MPSC actor that serializes all LedgerStore access.
//
// One background task owns the store and processes commands sequentially.
// Callers hold a cloneable `StoreServer` handle with explicit public methods.
// StoreServer does NOT implement LedgerStore — no component other than the
// internal MPSC consumer ever holds a raw store reference.
// Compound operations (load-modify-save) execute as single commands,
// preventing interleaving within this actor. Backends enforce cross-process safety.

use std::sync::Arc;

use tokio::sync::{broadcast, mpsc, oneshot};
use tracing::warn;
use unbill_event::ServiceEvent;
use unbill_model::error::{Result as DeviceResult, UnbillError};
use unbill_model::{
    Invitation, LedgerId, LedgerMeta, NewDevice, NodeId, SecretKey, StorageError, Timestamp,
};

use unbill_model::LedgerDoc;

use crate::{LedgerStore, StorageResult};
use std::collections::HashMap;

// sirno:witness:unbill-storage:begin
enum StoreCommand {
    RefreshEvents {
        events: broadcast::Sender<ServiceEvent>,
    },
    // --- Individual operations ---
    SaveLedgerMeta {
        meta: LedgerMeta,
        reply: oneshot::Sender<StorageResult<()>>,
    },
    ListLedgers {
        reply: oneshot::Sender<StorageResult<Vec<LedgerMeta>>>,
    },
    LoadLedger {
        ledger_id: String,
        reply: oneshot::Sender<StorageResult<Option<LedgerDoc>>>,
    },
    SaveLedger {
        ledger_id: String,
        doc: Box<LedgerDoc>,
        reply: oneshot::Sender<StorageResult<LedgerDoc>>,
    },
    ListDeviceLabels {
        reply: oneshot::Sender<StorageResult<HashMap<String, String>>>,
    },
    SetDeviceLabel {
        node_id: NodeId,
        label: Option<String>,
        reply: oneshot::Sender<StorageResult<()>>,
    },
    CreateSecretKey {
        reply: oneshot::Sender<StorageResult<()>>,
    },
    IsDeviceInitialized {
        reply: oneshot::Sender<StorageResult<bool>>,
    },
    GetDeviceId {
        reply: oneshot::Sender<StorageResult<NodeId>>,
    },
    GetSecretKey {
        reply: oneshot::Sender<StorageResult<SecretKey>>,
    },

    // --- Compound operations (atomic read-modify-write) ---
    AsymSync {
        ledger_id: String,
        bytes: Vec<u8>,
        reply: oneshot::Sender<DeviceResult<Option<Vec<u8>>>>,
    },
    CreateInvitation {
        ledger_id: LedgerId,
        device_id: NodeId,
        reply: oneshot::Sender<DeviceResult<String>>,
    },
    CollectPeers {
        self_id: NodeId,
        reply: oneshot::Sender<DeviceResult<Vec<NodeId>>>,
    },
    ConsumeInvitation {
        token: String,
        reply: oneshot::Sender<DeviceResult<Option<Invitation>>>,
    },
    AddDeviceToLedger {
        ledger_id: String,
        peer_node_id: NodeId,
        reply: oneshot::Sender<DeviceResult<Option<Vec<u8>>>>,
    },
    PersistJoinedLedger {
        doc_bytes: Vec<u8>,
        host_node_id: NodeId,
        label: Option<String>,
        reply: oneshot::Sender<DeviceResult<()>>,
    },
    MergeAndSaveLedger {
        ledger_id: String,
        doc: Box<LedgerDoc>,
        reply: oneshot::Sender<DeviceResult<()>>,
    },
}

pub struct StoreServer {
    tx: mpsc::Sender<StoreCommand>,
    events: broadcast::Sender<ServiceEvent>,
}

impl StoreServer {
    pub fn spawn(store: Arc<dyn LedgerStore>) -> Self {
        let (tx, rx) = mpsc::channel(64);
        let (events, _) = broadcast::channel(256);

        let mut inner_rx = store.subscribe();
        let fwd_tx = events.clone();
        let refresh_tx = tx.downgrade();
        tokio::spawn(async move {
            loop {
                match inner_rx.recv().await {
                    Ok(event) => {
                        let _ = fwd_tx.send(event);
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                    Err(broadcast::error::RecvError::Lagged(_)) => {
                        let Some(tx) = refresh_tx.upgrade() else {
                            break;
                        };
                        if tx
                            .send(StoreCommand::RefreshEvents {
                                events: fwd_tx.clone(),
                            })
                            .await
                            .is_err()
                        {
                            break;
                        }
                    }
                }
            }
        });

        tokio::spawn(Self::run(store, rx));

        Self { tx, events }
    }

    async fn run(store: Arc<dyn LedgerStore>, mut rx: mpsc::Receiver<StoreCommand>) {
        while let Some(cmd) = rx.recv().await {
            match cmd {
                StoreCommand::RefreshEvents { events } => {
                    match store.list_ledgers().await {
                        Ok(ledgers) => {
                            for meta in ledgers {
                                let _ = events.send(ServiceEvent::LedgerUpdated {
                                    ledger_id: meta.ledger_id.to_string(),
                                });
                            }
                        }
                        Err(error) => warn!(%error, "could not refresh ledger notifications"),
                    }
                    let _ = events.send(ServiceEvent::DeviceIdentityInitialized);
                    let _ = events.send(ServiceEvent::DeviceLabelsUpdated);
                    let _ = events.send(ServiceEvent::PendingInvitationsUpdated);
                }
                StoreCommand::SaveLedgerMeta { meta, reply } => {
                    if reply.send(store.save_ledger_meta(&meta).await).is_err() {
                        warn!("SaveLedgerMeta reply dropped (caller cancelled)");
                    }
                }
                StoreCommand::ListLedgers { reply } => {
                    if reply.send(store.list_ledgers().await).is_err() {
                        warn!("ListLedgers reply dropped (caller cancelled)");
                    }
                }
                StoreCommand::LoadLedger { ledger_id, reply } => {
                    if reply.send(store.load_ledger(&ledger_id).await).is_err() {
                        warn!(ledger_id, "LoadLedger reply dropped (caller cancelled)");
                    }
                }
                StoreCommand::SaveLedger {
                    ledger_id,
                    mut doc,
                    reply,
                } => {
                    let result = store.save_ledger(&ledger_id, &mut doc).await;
                    if reply.send(result.map(|()| *doc)).is_err() {
                        warn!(ledger_id, "SaveLedger reply dropped (caller cancelled)");
                    }
                }
                StoreCommand::ListDeviceLabels { reply } => {
                    if reply.send(store.list_device_labels().await).is_err() {
                        warn!("ListDeviceLabels reply dropped (caller cancelled)");
                    }
                }
                StoreCommand::SetDeviceLabel {
                    node_id,
                    label,
                    reply,
                } => {
                    if reply
                        .send(store.set_device_label(&node_id, label.as_deref()).await)
                        .is_err()
                    {
                        warn!("SetDeviceLabel reply dropped (caller cancelled)");
                    }
                }
                StoreCommand::CreateSecretKey { reply } => {
                    if reply.send(store.create_secret_key().await).is_err() {
                        warn!("CreateSecretKey reply dropped (caller cancelled)");
                    }
                }
                StoreCommand::IsDeviceInitialized { reply } => {
                    if reply.send(store.is_device_initialized().await).is_err() {
                        warn!("IsDeviceInitialized reply dropped (caller cancelled)");
                    }
                }
                StoreCommand::GetDeviceId { reply } => {
                    if reply.send(store.get_device_id().await).is_err() {
                        warn!("GetDeviceId reply dropped (caller cancelled)");
                    }
                }
                StoreCommand::GetSecretKey { reply } => {
                    if reply.send(store.get_secret_key().await).is_err() {
                        warn!("GetSecretKey reply dropped (caller cancelled)");
                    }
                }
                StoreCommand::AsymSync {
                    ledger_id,
                    bytes,
                    reply,
                } => {
                    if reply
                        .send(Self::do_asym_sync(&*store, &ledger_id, bytes).await)
                        .is_err()
                    {
                        warn!(ledger_id, "AsymSync reply dropped (caller cancelled)");
                    }
                }
                StoreCommand::CreateInvitation {
                    ledger_id,
                    device_id,
                    reply,
                } => {
                    if reply
                        .send(Self::do_create_invitation(&*store, ledger_id, device_id).await)
                        .is_err()
                    {
                        warn!(%ledger_id, "CreateInvitation reply dropped (caller cancelled)");
                    }
                }
                StoreCommand::CollectPeers { self_id, reply } => {
                    if reply
                        .send(Self::do_collect_peers(&*store, &self_id).await)
                        .is_err()
                    {
                        warn!("CollectPeers reply dropped (caller cancelled)");
                    }
                }
                StoreCommand::ConsumeInvitation { token, reply } => {
                    if reply
                        .send(Self::do_consume_invitation(&*store, &token).await)
                        .is_err()
                    {
                        warn!(token, "ConsumeInvitation reply dropped (caller cancelled)");
                    }
                }
                StoreCommand::AddDeviceToLedger {
                    ledger_id,
                    peer_node_id,
                    reply,
                } => {
                    if reply
                        .send(
                            Self::do_add_device_to_ledger(&*store, &ledger_id, peer_node_id).await,
                        )
                        .is_err()
                    {
                        warn!(
                            ledger_id,
                            "AddDeviceToLedger reply dropped (caller cancelled)"
                        );
                    }
                }
                StoreCommand::PersistJoinedLedger {
                    doc_bytes,
                    host_node_id,
                    label,
                    reply,
                } => {
                    if reply
                        .send(
                            Self::do_persist_joined_ledger(&*store, doc_bytes, host_node_id, label)
                                .await,
                        )
                        .is_err()
                    {
                        warn!("PersistJoinedLedger reply dropped (caller cancelled)");
                    }
                }
                StoreCommand::MergeAndSaveLedger {
                    ledger_id,
                    mut doc,
                    reply,
                } => {
                    if reply
                        .send(Self::do_merge_and_save(&*store, &ledger_id, &mut doc).await)
                        .is_err()
                    {
                        warn!(
                            ledger_id,
                            "MergeAndSaveLedger reply dropped (caller cancelled)"
                        );
                    }
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // Compound operation implementations (run inside the MPSC consumer)
    // -----------------------------------------------------------------------

    async fn do_asym_sync(
        store: &dyn LedgerStore,
        ledger_id: &str,
        bytes: Vec<u8>,
    ) -> DeviceResult<Option<Vec<u8>>> {
        use automerge::sync::Message;
        let client_msg =
            Message::decode(&bytes).map_err(|e| UnbillError::Automerge(e.to_string()))?;
        let mut doc = store
            .load_ledger(ledger_id)
            .await?
            .unwrap_or_else(LedgerDoc::empty);
        let mut sync_state = automerge::sync::State::new();
        let heads_before = doc.heads();
        doc.receive_sync_message(&mut sync_state, client_msg)
            .map_err(|e| UnbillError::Automerge(e.to_string()))?;
        if doc.heads() != heads_before {
            store.save_ledger(ledger_id, &mut doc).await?;
        }
        Ok(doc
            .generate_sync_message(&mut sync_state)
            .map(|m| m.encode()))
    }

    async fn do_create_invitation(
        store: &dyn LedgerStore,
        ledger_id: LedgerId,
        device_id: NodeId,
    ) -> DeviceResult<String> {
        let id_str = ledger_id.to_string();
        let _ = store
            .load_ledger(&id_str)
            .await?
            .ok_or(UnbillError::LedgerNotFound(id_str))?;
        let now = Timestamp::now()?;
        let invitation = store
            .create_invitation(
                ledger_id,
                &device_id,
                now,
                Timestamp::from_millis(now.as_millis() + 24 * 3600 * 1000),
            )
            .await?;
        let token = invitation.token;
        Ok(format!(
            "unbill://join/{}/{}/{}",
            ledger_id, device_id, token
        ))
    }

    async fn do_collect_peers(
        store: &dyn LedgerStore,
        self_id: &NodeId,
    ) -> DeviceResult<Vec<NodeId>> {
        let metas = store.list_ledgers().await?;
        let mut peers = Vec::new();
        for meta in metas {
            let id = meta.ledger_id.to_string();
            if let Some(doc) = store.load_ledger(&id).await?
                && let Ok(devices) = doc.list_devices()
            {
                for device in devices {
                    if device.node_id != *self_id && !peers.contains(&device.node_id) {
                        peers.push(device.node_id);
                    }
                }
            }
        }
        Ok(peers)
    }

    async fn do_consume_invitation(
        store: &dyn LedgerStore,
        token: &str,
    ) -> DeviceResult<Option<Invitation>> {
        Ok(store.consume_invitation(token).await?)
    }

    async fn do_add_device_to_ledger(
        store: &dyn LedgerStore,
        ledger_id: &str,
        peer_node_id: NodeId,
    ) -> DeviceResult<Option<Vec<u8>>> {
        let Some(mut doc) = store.load_ledger(ledger_id).await? else {
            return Ok(None);
        };
        doc.add_device(
            NewDevice {
                node_id: peer_node_id,
            },
            Timestamp::now()?,
        )?;
        store.save_ledger(ledger_id, &mut doc).await?;
        Ok(Some(doc.save()))
    }

    async fn do_merge_and_save(
        store: &dyn LedgerStore,
        ledger_id: &str,
        synced_doc: &mut LedgerDoc,
    ) -> DeviceResult<()> {
        match store.load_ledger(ledger_id).await? {
            Some(mut current) => {
                current.merge(synced_doc)?;
                store.save_ledger(ledger_id, &mut current).await?;
            }
            None => {
                store.save_ledger(ledger_id, synced_doc).await?;
            }
        }
        Ok(())
    }

    async fn do_persist_joined_ledger(
        store: &dyn LedgerStore,
        doc_bytes: Vec<u8>,
        host_node_id: NodeId,
        label: Option<String>,
    ) -> DeviceResult<()> {
        let mut doc = LedgerDoc::from_bytes(&doc_bytes)?;
        let ledger = doc.get_ledger()?;
        let meta = LedgerMeta {
            ledger_id: ledger.ledger_id,
            name: ledger.name.clone(),
            currency: ledger.currency,
            created_at: ledger.created_at,
            updated_at: Timestamp::now()?,
        };
        store.save_ledger_meta(&meta).await?;
        store
            .save_ledger(&meta.ledger_id.to_string(), &mut doc)
            .await?;
        if let Some(label) = label {
            store.set_device_label(&host_node_id, Some(&label)).await?;
        }
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Public individual methods
    // -----------------------------------------------------------------------

    pub async fn save_ledger_meta(&self, meta: &LedgerMeta) -> StorageResult<()> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(StoreCommand::SaveLedgerMeta {
                meta: meta.clone(),
                reply: tx,
            })
            .await
            .map_err(|_| StorageError::ChannelClosed)?;
        rx.await.map_err(|_| StorageError::ChannelClosed)?
    }

    pub async fn list_ledgers(&self) -> StorageResult<Vec<LedgerMeta>> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(StoreCommand::ListLedgers { reply: tx })
            .await
            .map_err(|_| StorageError::ChannelClosed)?;
        rx.await.map_err(|_| StorageError::ChannelClosed)?
    }

    pub async fn load_ledger(&self, ledger_id: &str) -> StorageResult<Option<LedgerDoc>> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(StoreCommand::LoadLedger {
                ledger_id: ledger_id.to_owned(),
                reply: tx,
            })
            .await
            .map_err(|_| StorageError::ChannelClosed)?;
        rx.await.map_err(|_| StorageError::ChannelClosed)?
    }

    pub async fn save_ledger(&self, ledger_id: &str, doc: &mut LedgerDoc) -> StorageResult<()> {
        let owned = LedgerDoc::from_bytes(&doc.save())
            .map_err(|e| StorageError::Serialization(e.to_string()))?;
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(StoreCommand::SaveLedger {
                ledger_id: ledger_id.to_owned(),
                doc: Box::new(owned),
                reply: tx,
            })
            .await
            .map_err(|_| StorageError::ChannelClosed)?;
        let result = rx.await.map_err(|_| StorageError::ChannelClosed)?;
        match result {
            Ok(returned_doc) => {
                *doc = returned_doc;
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    pub async fn list_device_labels(&self) -> StorageResult<HashMap<String, String>> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(StoreCommand::ListDeviceLabels { reply: tx })
            .await
            .map_err(|_| StorageError::ChannelClosed)?;
        rx.await.map_err(|_| StorageError::ChannelClosed)?
    }
    pub async fn set_device_label(
        &self,
        node_id: &NodeId,
        label: Option<&str>,
    ) -> StorageResult<()> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(StoreCommand::SetDeviceLabel {
                node_id: node_id.clone(),
                label: label.map(str::to_owned),
                reply: tx,
            })
            .await
            .map_err(|_| StorageError::ChannelClosed)?;
        rx.await.map_err(|_| StorageError::ChannelClosed)?
    }

    pub async fn create_secret_key(&self) -> StorageResult<()> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(StoreCommand::CreateSecretKey { reply: tx })
            .await
            .map_err(|_| StorageError::ChannelClosed)?;
        rx.await.map_err(|_| StorageError::ChannelClosed)?
    }

    pub async fn is_device_initialized(&self) -> StorageResult<bool> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(StoreCommand::IsDeviceInitialized { reply: tx })
            .await
            .map_err(|_| StorageError::ChannelClosed)?;
        rx.await.map_err(|_| StorageError::ChannelClosed)?
    }

    pub async fn get_device_id(&self) -> StorageResult<NodeId> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(StoreCommand::GetDeviceId { reply: tx })
            .await
            .map_err(|_| StorageError::ChannelClosed)?;
        rx.await.map_err(|_| StorageError::ChannelClosed)?
    }

    pub async fn get_secret_key(&self) -> StorageResult<SecretKey> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(StoreCommand::GetSecretKey { reply: tx })
            .await
            .map_err(|_| StorageError::ChannelClosed)?;
        rx.await.map_err(|_| StorageError::ChannelClosed)?
    }

    pub fn subscribe(&self) -> broadcast::Receiver<ServiceEvent> {
        self.events.subscribe()
    }

    // -----------------------------------------------------------------------
    // Public compound methods
    // -----------------------------------------------------------------------

    pub async fn asym_sync(
        &self,
        ledger_id: LedgerId,
        bytes: Vec<u8>,
    ) -> DeviceResult<Option<Vec<u8>>> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(StoreCommand::AsymSync {
                ledger_id: ledger_id.to_string(),
                bytes,
                reply: tx,
            })
            .await
            .map_err(|_| UnbillError::Storage(StorageError::ChannelClosed))?;
        rx.await
            .map_err(|_| UnbillError::Storage(StorageError::ChannelClosed))?
    }

    pub async fn create_invitation(
        &self,
        ledger_id: LedgerId,
        device_id: NodeId,
    ) -> DeviceResult<String> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(StoreCommand::CreateInvitation {
                ledger_id,
                device_id,
                reply: tx,
            })
            .await
            .map_err(|_| UnbillError::Storage(StorageError::ChannelClosed))?;
        rx.await
            .map_err(|_| UnbillError::Storage(StorageError::ChannelClosed))?
    }

    pub async fn collect_peers(&self, self_id: NodeId) -> DeviceResult<Vec<NodeId>> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(StoreCommand::CollectPeers { self_id, reply: tx })
            .await
            .map_err(|_| UnbillError::Storage(StorageError::ChannelClosed))?;
        rx.await
            .map_err(|_| UnbillError::Storage(StorageError::ChannelClosed))?
    }

    pub async fn consume_invitation(&self, token: &str) -> DeviceResult<Option<Invitation>> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(StoreCommand::ConsumeInvitation {
                token: token.to_owned(),
                reply: tx,
            })
            .await
            .map_err(|_| UnbillError::Storage(StorageError::ChannelClosed))?;
        rx.await
            .map_err(|_| UnbillError::Storage(StorageError::ChannelClosed))?
    }

    pub async fn add_device_to_ledger(
        &self,
        ledger_id: &str,
        peer_node_id: NodeId,
    ) -> DeviceResult<Option<Vec<u8>>> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(StoreCommand::AddDeviceToLedger {
                ledger_id: ledger_id.to_owned(),
                peer_node_id,
                reply: tx,
            })
            .await
            .map_err(|_| UnbillError::Storage(StorageError::ChannelClosed))?;
        rx.await
            .map_err(|_| UnbillError::Storage(StorageError::ChannelClosed))?
    }

    pub async fn merge_and_save_ledger(&self, ledger_id: &str, doc: LedgerDoc) -> DeviceResult<()> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(StoreCommand::MergeAndSaveLedger {
                ledger_id: ledger_id.to_owned(),
                doc: Box::new(doc),
                reply: tx,
            })
            .await
            .map_err(|_| UnbillError::Storage(StorageError::ChannelClosed))?;
        rx.await
            .map_err(|_| UnbillError::Storage(StorageError::ChannelClosed))?
    }

    pub async fn persist_joined_ledger(
        &self,
        doc_bytes: Vec<u8>,
        host_node_id: NodeId,
        label: Option<String>,
    ) -> DeviceResult<()> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(StoreCommand::PersistJoinedLedger {
                doc_bytes,
                host_node_id,
                label,
                reply: tx,
            })
            .await
            .map_err(|_| UnbillError::Storage(StorageError::ChannelClosed))?;
        rx.await
            .map_err(|_| UnbillError::Storage(StorageError::ChannelClosed))?
    }
}
// sirno:witness:unbill-storage:end

// Tests live in unbill-device (which has access to InMemoryStore).
