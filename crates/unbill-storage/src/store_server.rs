// StoreServer: MPSC actor that serializes all LedgerStore access.
//
// One background task owns the store and processes commands sequentially.
// Callers hold a cloneable `StoreServer` handle with explicit public methods.
// StoreServer does NOT implement LedgerStore — no component other than the
// internal MPSC consumer ever holds a raw store reference.
// Compound operations (load-modify-save) execute as single commands,
// preventing interleaving within this actor. Backends enforce cross-process safety.

use std::sync::Arc;
use std::{future::Future, pin::Pin};

use tokio::sync::{broadcast, mpsc, oneshot};
use tracing::warn;
use unbill_event::ServiceEvent;
use unbill_model::error::{Result as DeviceResult, UnbillError};
use unbill_model::{
    Invitation, LedgerId, LedgerMeta, NewDevice, NodeId, SecretKey, StorageError, Timestamp,
};

use unbill_model::LedgerDoc;

use crate::{AccountRecord, LedgerStore, StorageResult};

type StoreAction =
    Box<dyn FnOnce(Arc<dyn LedgerStore>) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send>;

// sirno:witness:unbill-storage:begin
enum StoreCommand {
    Account(StoreAction),
    RefreshEvents {
        events: broadcast::Sender<ServiceEvent>,
    },
    // --- Individual operations ---
    SaveLedgerMeta {
        meta: LedgerMeta,
        reply: oneshot::Sender<StorageResult<()>>,
    },
    SetLedgerArchived {
        ledger_id: LedgerId,
        archived: bool,
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
        device: NewDevice,
        reply: oneshot::Sender<DeviceResult<Option<Vec<u8>>>>,
    },
    PersistJoinedLedger {
        doc_bytes: Vec<u8>,
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
                StoreCommand::Account(action) => action(Arc::clone(&store)).await,
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
                    let _ = events.send(ServiceEvent::PendingInvitationsUpdated);
                }
                StoreCommand::SaveLedgerMeta { meta, reply } => {
                    if reply.send(store.save_ledger_meta(&meta).await).is_err() {
                        warn!("SaveLedgerMeta reply dropped (caller cancelled)");
                    }
                }
                StoreCommand::SetLedgerArchived {
                    ledger_id,
                    archived,
                    reply,
                } => {
                    if reply
                        .send(store.set_ledger_archived(ledger_id, archived).await)
                        .is_err()
                    {
                        warn!("SetLedgerArchived reply dropped (caller cancelled)");
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
                    device,
                    reply,
                } => {
                    if reply
                        .send(Self::do_add_device_to_ledger(&*store, &ledger_id, device).await)
                        .is_err()
                    {
                        warn!(
                            ledger_id,
                            "AddDeviceToLedger reply dropped (caller cancelled)"
                        );
                    }
                }
                StoreCommand::PersistJoinedLedger { doc_bytes, reply } => {
                    if reply
                        .send(Self::do_persist_joined_ledger(&*store, doc_bytes).await)
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
        #[allow(
            clippy::arithmetic_side_effects,
            reason = "Adding one day to a realistic system clock is far below the i64 millisecond limit"
        )]
        let expires_at = Timestamp::from_millis(now.as_millis() + 24 * 3600 * 1000);
        let invitation = store
            .create_invitation(ledger_id, &device_id, now, expires_at)
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
        device: NewDevice,
    ) -> DeviceResult<Option<Vec<u8>>> {
        let Some(mut doc) = store.load_ledger(ledger_id).await? else {
            return Ok(None);
        };
        if !doc
            .list_devices()?
            .iter()
            .any(|existing| existing.node_id == device.node_id)
        {
            doc.add_device(device, Timestamp::now()?)?;
            store.save_ledger(ledger_id, &mut doc).await?;
        }
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
    ) -> DeviceResult<()> {
        let mut doc = LedgerDoc::from_bytes(&doc_bytes)?;
        let ledger = doc.get_ledger()?;
        let id = ledger.ledger_id.to_string();
        if let Some(mut current) = store.load_ledger(&id).await? {
            current.merge(&mut doc)?;
            doc = current;
        }
        store.save_ledger(&id, &mut doc).await?;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Public individual methods
    // -----------------------------------------------------------------------

    async fn account_call<T, F, Fut>(&self, operation: F) -> StorageResult<T>
    where
        T: Send + 'static,
        F: FnOnce(Arc<dyn LedgerStore>) -> Fut + Send + 'static,
        Fut: Future<Output = StorageResult<T>> + Send + 'static,
    {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(StoreCommand::Account(Box::new(move |store| {
                Box::pin(async move {
                    if tx.send(operation(store).await).is_err() {
                        warn!("account operation reply dropped (caller cancelled)");
                    }
                })
            })))
            .await
            .map_err(|_| StorageError::ChannelClosed)?;
        rx.await.map_err(|_| StorageError::ChannelClosed)?
    }

    pub async fn create_account(&self, name: &str, password_hash: &str) -> StorageResult<()> {
        let (name, hash) = (name.to_owned(), password_hash.to_owned());
        self.account_call(move |store| async move { store.create_account(&name, &hash).await })
            .await
    }
    pub async fn get_account(&self, name: &str) -> StorageResult<Option<AccountRecord>> {
        let name = name.to_owned();
        self.account_call(move |store| async move { store.get_account(&name).await })
            .await
    }
    pub async fn list_accounts(&self) -> StorageResult<Vec<String>> {
        self.account_call(|store| async move { store.list_accounts().await })
            .await
    }
    pub async fn set_account_password(&self, name: &str, password_hash: &str) -> StorageResult<()> {
        let (name, hash) = (name.to_owned(), password_hash.to_owned());
        self.account_call(
            move |store| async move { store.set_account_password(&name, &hash).await },
        )
        .await
    }
    pub async fn delete_account(&self, name: &str) -> StorageResult<()> {
        let name = name.to_owned();
        self.account_call(move |store| async move { store.delete_account(&name).await })
            .await
    }
    pub async fn create_session(&self, token_hash: &str, account_name: &str) -> StorageResult<()> {
        let (hash, name) = (token_hash.to_owned(), account_name.to_owned());
        self.account_call(move |store| async move { store.create_session(&hash, &name).await })
            .await
    }
    pub async fn resolve_session(&self, token_hash: &str) -> StorageResult<Option<String>> {
        let hash = token_hash.to_owned();
        self.account_call(move |store| async move { store.resolve_session(&hash).await })
            .await
    }
    pub async fn delete_session(&self, token_hash: &str) -> StorageResult<()> {
        let hash = token_hash.to_owned();
        self.account_call(move |store| async move { store.delete_session(&hash).await })
            .await
    }
    pub async fn grant_ledger(&self, name: &str, id: LedgerId) -> StorageResult<()> {
        let name = name.to_owned();
        self.account_call(move |store| async move { store.grant_ledger(&name, id).await })
            .await
    }
    pub async fn revoke_ledger(&self, name: &str, id: LedgerId) -> StorageResult<()> {
        let name = name.to_owned();
        self.account_call(move |store| async move { store.revoke_ledger(&name, id).await })
            .await
    }
    pub async fn account_has_ledger(&self, name: &str, id: LedgerId) -> StorageResult<bool> {
        let name = name.to_owned();
        self.account_call(move |store| async move { store.account_has_ledger(&name, id).await })
            .await
    }
    pub async fn list_account_ledgers(&self, name: &str) -> StorageResult<Vec<LedgerId>> {
        let name = name.to_owned();
        self.account_call(move |store| async move { store.list_account_ledgers(&name).await })
            .await
    }

    pub async fn list_account_ledger_metas(&self, name: &str) -> StorageResult<Vec<LedgerMeta>> {
        let name = name.to_owned();
        self.account_call(move |store| async move {
            if store.get_account(&name).await?.is_none() {
                return Err(StorageError::Unauthorized);
            }
            let ids = store.list_account_ledgers(&name).await?;
            Ok(store
                .list_ledgers()
                .await?
                .into_iter()
                .filter(|meta| ids.contains(&meta.ledger_id))
                .collect())
        })
        .await
    }

    /// Serialize the grant check with metadata creation/update so two accounts
    /// cannot both claim a new ledger ID during a concurrent first write.
    pub async fn save_account_ledger_meta(
        &self,
        name: &str,
        meta: &LedgerMeta,
    ) -> StorageResult<()> {
        let (name, meta) = (name.to_owned(), meta.clone());
        self.account_call(move |store| async move {
            if store.get_account(&name).await?.is_none() {
                return Err(StorageError::Unauthorized);
            }
            let existing = store
                .list_ledgers()
                .await?
                .iter()
                .any(|row| row.ledger_id == meta.ledger_id);
            if existing && !store.account_has_ledger(&name, meta.ledger_id).await? {
                return Err(StorageError::Unauthorized);
            }
            store.save_ledger_meta(&meta).await?;
            if !existing {
                store.grant_ledger(&name, meta.ledger_id).await?;
            }
            Ok(())
        })
        .await
    }

    pub async fn set_account_ledger_archived(
        &self,
        name: &str,
        id: LedgerId,
        archived: bool,
    ) -> StorageResult<()> {
        let name = name.to_owned();
        self.account_call(move |store| async move {
            if !store.account_has_ledger(&name, id).await? {
                return Err(StorageError::Unauthorized);
            }
            store.set_ledger_archived(id, archived).await
        })
        .await
    }

    pub async fn account_asym_sync(
        &self,
        name: &str,
        id: LedgerId,
        bytes: Vec<u8>,
    ) -> DeviceResult<Option<Vec<u8>>> {
        let name = name.to_owned();
        self.account_call(move |store| async move {
            if !store.account_has_ledger(&name, id).await? {
                return Err(StorageError::Unauthorized);
            }
            Ok(Self::do_asym_sync(&*store, &id.to_string(), bytes).await)
        })
        .await
        .map_err(|error| match error {
            StorageError::Unauthorized => UnbillError::NotAuthorized,
            other => UnbillError::Storage(other),
        })?
    }

    pub async fn account_create_invitation(
        &self,
        name: &str,
        id: LedgerId,
        device_id: NodeId,
    ) -> DeviceResult<String> {
        let name = name.to_owned();
        self.account_call(move |store| async move {
            if !store.account_has_ledger(&name, id).await? {
                return Err(StorageError::Unauthorized);
            }
            Ok(Self::do_create_invitation(&*store, id, device_id).await)
        })
        .await
        .map_err(|error| match error {
            StorageError::Unauthorized => UnbillError::NotAuthorized,
            other => UnbillError::Storage(other),
        })?
    }

    pub async fn share_account_ledger(
        &self,
        source: &str,
        target: &str,
        id: LedgerId,
    ) -> StorageResult<()> {
        let (source, target) = (source.to_owned(), target.to_owned());
        self.account_call(move |store| async move {
            if !store.account_has_ledger(&source, id).await? {
                return Err(StorageError::Unauthorized);
            }
            if store.get_account(&target).await?.is_none() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "target account not found",
                )
                .into());
            }
            store.grant_ledger(&target, id).await
        })
        .await
    }

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

    pub async fn set_ledger_archived(
        &self,
        ledger_id: LedgerId,
        archived: bool,
    ) -> StorageResult<()> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(StoreCommand::SetLedgerArchived {
                ledger_id,
                archived,
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
        device: NewDevice,
    ) -> DeviceResult<Option<Vec<u8>>> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(StoreCommand::AddDeviceToLedger {
                ledger_id: ledger_id.to_owned(),
                device,
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

    pub async fn persist_joined_ledger(&self, doc_bytes: Vec<u8>) -> DeviceResult<()> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(StoreCommand::PersistJoinedLedger {
                doc_bytes,
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
