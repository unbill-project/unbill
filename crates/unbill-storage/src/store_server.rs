// StoreServer: MPSC actor that serializes all LedgerStore access.
//
// One background task owns the store and processes commands sequentially.
// Callers hold a cloneable `StoreServer` handle with explicit public methods.
// StoreServer does NOT implement LedgerStore — no component other than the
// internal MPSC consumer ever holds a raw store reference.
// Compound operations (load-modify-save) execute as single commands,
// guaranteeing atomicity against concurrent access.

use std::sync::Arc;

use tokio::sync::{broadcast, mpsc, oneshot};
use tracing::warn;
use unbill_event::ServiceEvent;
use unbill_model::error::{Result as DeviceResult, UnbillError};
use unbill_model::{
    Invitation, InviteToken, LedgerId, LedgerMeta, NewDevice, NodeId, SecretKey, StorageError,
    Timestamp,
};

use crate::{
    LedgerDoc, LedgerStore, StorageResult, load_device_labels, load_pending_invitations,
    save_device_labels, save_pending_invitations,
};

// sirno:witness:store-server:begin
enum StoreCommand {
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
    DeleteLedger {
        ledger_id: String,
        reply: oneshot::Sender<StorageResult<()>>,
    },
    LoadDeviceMeta {
        key: String,
        reply: oneshot::Sender<StorageResult<Option<Vec<u8>>>>,
    },
    SaveDeviceMeta {
        key: String,
        value: Vec<u8>,
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
        tokio::spawn(async move {
            while let Ok(evt) = inner_rx.recv().await {
                if fwd_tx.send(evt).is_err() {
                    break; // No subscribers, stop forwarding
                }
            }
        });

        tokio::spawn(Self::run(store, rx));

        Self { tx, events }
    }

    async fn run(store: Arc<dyn LedgerStore>, mut rx: mpsc::Receiver<StoreCommand>) {
        while let Some(cmd) = rx.recv().await {
            match cmd {
                StoreCommand::SaveLedgerMeta { meta, reply } => {
                    if reply.send(store.save_ledger_meta(&meta).await).is_err() {
                        warn!("store command reply dropped");
                    }
                }
                StoreCommand::ListLedgers { reply } => {
                    if reply.send(store.list_ledgers().await).is_err() {
                        warn!("store command reply dropped");
                    }
                }
                StoreCommand::LoadLedger { ledger_id, reply } => {
                    if reply.send(store.load_ledger(&ledger_id).await).is_err() {
                        warn!("store command reply dropped");
                    }
                }
                StoreCommand::SaveLedger {
                    ledger_id,
                    mut doc,
                    reply,
                } => {
                    let result = store.save_ledger(&ledger_id, &mut doc).await;
                    if reply.send(result.map(|()| *doc)).is_err() {
                        warn!("store command reply dropped");
                    }
                }
                StoreCommand::DeleteLedger { ledger_id, reply } => {
                    if reply.send(store.delete_ledger(&ledger_id).await).is_err() {
                        warn!("store command reply dropped");
                    }
                }
                StoreCommand::LoadDeviceMeta { key, reply } => {
                    if reply.send(store.load_device_meta(&key).await).is_err() {
                        warn!("store command reply dropped");
                    }
                }
                StoreCommand::SaveDeviceMeta { key, value, reply } => {
                    if reply
                        .send(store.save_device_meta(&key, &value).await)
                        .is_err()
                    {
                        warn!("store command reply dropped");
                    }
                }
                StoreCommand::CreateSecretKey { reply } => {
                    if reply.send(store.create_secret_key().await).is_err() {
                        warn!("store command reply dropped");
                    }
                }
                StoreCommand::IsDeviceInitialized { reply } => {
                    if reply.send(store.is_device_initialized().await).is_err() {
                        warn!("store command reply dropped");
                    }
                }
                StoreCommand::GetDeviceId { reply } => {
                    if reply.send(store.get_device_id().await).is_err() {
                        warn!("store command reply dropped");
                    }
                }
                StoreCommand::GetSecretKey { reply } => {
                    if reply.send(store.get_secret_key().await).is_err() {
                        warn!("store command reply dropped");
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
                        warn!("store command reply dropped");
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
                        warn!("store command reply dropped");
                    }
                }
                StoreCommand::CollectPeers { self_id, reply } => {
                    if reply
                        .send(Self::do_collect_peers(&*store, &self_id).await)
                        .is_err()
                    {
                        warn!("store command reply dropped");
                    }
                }
                StoreCommand::ConsumeInvitation { token, reply } => {
                    if reply
                        .send(Self::do_consume_invitation(&*store, &token).await)
                        .is_err()
                    {
                        warn!("store command reply dropped");
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
                        warn!("store command reply dropped");
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
                        warn!("store command reply dropped");
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
                        warn!("store command reply dropped");
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
        let token = InviteToken::generate();
        let now = Timestamp::now();
        let invitation = Invitation {
            token: token.clone(),
            ledger_id,
            created_by_device: device_id.clone(),
            created_at: now,
            expires_at: Timestamp::from_millis(now.as_millis() + 24 * 3600 * 1000),
        };
        let mut map = load_pending_invitations(store).await?;
        map.insert(token.to_string(), invitation);
        save_pending_invitations(store, &map).await?;
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
        let mut map = load_pending_invitations(store).await?;
        let inv = map.remove(token);
        save_pending_invitations(store, &map).await?;
        Ok(inv)
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
            Timestamp::now(),
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
            updated_at: Timestamp::now(),
        };
        store.save_ledger_meta(&meta).await?;
        store
            .save_ledger(&meta.ledger_id.to_string(), &mut doc)
            .await?;
        if let Some(label) = label {
            let mut labels = load_device_labels(store).await?;
            labels.insert(host_node_id.to_string(), label);
            save_device_labels(store, &labels).await?;
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
        let owned = std::mem::replace(doc, LedgerDoc::empty());
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

    pub async fn delete_ledger(&self, ledger_id: &str) -> StorageResult<()> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(StoreCommand::DeleteLedger {
                ledger_id: ledger_id.to_owned(),
                reply: tx,
            })
            .await
            .map_err(|_| StorageError::ChannelClosed)?;
        rx.await.map_err(|_| StorageError::ChannelClosed)?
    }

    pub async fn load_device_meta(&self, key: &str) -> StorageResult<Option<Vec<u8>>> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(StoreCommand::LoadDeviceMeta {
                key: key.to_owned(),
                reply: tx,
            })
            .await
            .map_err(|_| StorageError::ChannelClosed)?;
        rx.await.map_err(|_| StorageError::ChannelClosed)?
    }

    pub async fn save_device_meta(&self, key: &str, value: &[u8]) -> StorageResult<()> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(StoreCommand::SaveDeviceMeta {
                key: key.to_owned(),
                value: value.to_vec(),
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
// sirno:witness:store-server:end

// Tests live in unbill-device (which has access to InMemoryStore).
