// unbill-device: device-side implementation of the AsymChannel contract.
//
// Holds a local LedgerStore and provides invitation management, peer sync,
// and the Automerge sync server-side handler.  Does not know about the
// AsymChannel trait — LocalAsymChannel wraps this and wires up the trait.

use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier, password_hash::SaltString};
use rand::TryRng as _;
use sha2::{Digest, Sha256};
use std::sync::Arc;

pub use unbill_event::ServiceEvent;
pub use unbill_storage::StoreServer;

use tokio::io::AsyncWriteExt;
use tokio::sync::{broadcast, watch};
use tracing::warn;
use unbill_model::error::{Result, UnbillError};
use unbill_model::{LedgerId, LedgerMeta, NodeId, StorageError, Timestamp};
use unbill_symmetric_channel::{
    ALPN_JOIN, ALPN_SYNC, JoinError, JoinReply, JoinRequest, JoinResponse, UnbillEndpoint,
    finish_sync_session, read_msg, run_sync_session, write_msg,
};

// sirno:witness:unbill-device:begin
pub struct UnbillDevice {
    server: Arc<StoreServer>,
    device_id: NodeId,
    endpoint: UnbillEndpoint,
    ble: watch::Sender<unbill_model::BleSnapshot>,
}

/// A trusted, in-process account context. External transports must derive the
/// account name from a verified session; accepting a client-supplied name is unsafe.
pub struct AccountView {
    device: Arc<UnbillDevice>,
    account_name: String,
}

fn password_hash(password: &str) -> Result<String> {
    let mut bytes = [0u8; 16];
    rand::rngs::SysRng
        .try_fill_bytes(&mut bytes)
        .map_err(|e| UnbillError::Validation(e.to_string()))?;
    let salt =
        SaltString::encode_b64(&bytes).map_err(|e| UnbillError::Validation(e.to_string()))?;
    Ok(Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| UnbillError::Validation(e.to_string()))?
        .to_string())
}

fn session_digest(token: &str) -> String {
    Sha256::digest(token.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn validate_account_name(name: &str) -> Result<()> {
    if name.is_empty()
        || name.len() > 100
        || name.trim() != name
        || name.chars().any(char::is_control)
    {
        return Err(UnbillError::Validation(
            "account name must be 1–100 characters without surrounding space or controls".into(),
        ));
    }
    Ok(())
}

impl UnbillDevice {
    pub async fn register(&self, name: &str, password: &str) -> Result<()> {
        validate_account_name(name)?;
        if password.is_empty() {
            return Err(UnbillError::Validation("password cannot be empty".into()));
        }
        self.server
            .create_account(name, &password_hash(password)?)
            .await?;
        Ok(())
    }

    pub async fn login(&self, name: &str, password: &str) -> Result<String> {
        let account = self
            .server
            .get_account(name)
            .await?
            .ok_or(UnbillError::NotAuthorized)?;
        let parsed = PasswordHash::new(&account.password_hash)
            .map_err(|e| UnbillError::Validation(e.to_string()))?;
        Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .map_err(|_| UnbillError::NotAuthorized)?;
        let mut bytes = [0u8; 32];
        rand::rngs::SysRng
            .try_fill_bytes(&mut bytes)
            .map_err(|e| UnbillError::Validation(e.to_string()))?;
        let token: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
        self.server
            .create_session(&session_digest(&token), name)
            .await?;
        Ok(token)
    }

    pub async fn session_account(&self, token: &str) -> Result<String> {
        if token.len() != 64 || !token.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(UnbillError::NotAuthorized);
        }
        self.server
            .resolve_session(&session_digest(token))
            .await?
            .ok_or(UnbillError::NotAuthorized)
    }

    pub async fn logout(&self, token: &str) -> Result<()> {
        self.server.delete_session(&session_digest(token)).await?;
        Ok(())
    }

    pub async fn change_password(
        &self,
        token: &str,
        old_password: &str,
        new_password: &str,
    ) -> Result<()> {
        if new_password.is_empty() {
            return Err(UnbillError::Validation("password cannot be empty".into()));
        }
        let name = self.session_account(token).await?;
        let account = self
            .server
            .get_account(&name)
            .await?
            .ok_or(UnbillError::NotAuthorized)?;
        let parsed = PasswordHash::new(&account.password_hash)
            .map_err(|e| UnbillError::Validation(e.to_string()))?;
        Argon2::default()
            .verify_password(old_password.as_bytes(), &parsed)
            .map_err(|_| UnbillError::NotAuthorized)?;
        self.server
            .set_account_password(&name, &password_hash(new_password)?)
            .await?;
        Ok(())
    }

    /// Trusted local maintenance entry points; never expose them on RPC or HTTP.
    pub async fn delete_account(&self, name: &str) -> Result<()> {
        self.server.delete_account(name).await?;
        Ok(())
    }
    pub async fn reset_account_password(&self, name: &str, new_password: &str) -> Result<()> {
        if new_password.is_empty() {
            return Err(UnbillError::Validation("password cannot be empty".into()));
        }
        self.server
            .set_account_password(name, &password_hash(new_password)?)
            .await?;
        Ok(())
    }
    pub async fn revoke_account_ledger(&self, name: &str, id: LedgerId) -> Result<()> {
        self.server.revoke_ledger(name, id).await?;
        Ok(())
    }

    pub async fn account_view(self: &Arc<Self>, account_name: &str) -> Result<AccountView> {
        if self.server.get_account(account_name).await?.is_none() {
            return Err(UnbillError::NotAuthorized);
        }
        Ok(AccountView {
            device: Arc::clone(self),
            account_name: account_name.to_owned(),
        })
    }

    pub async fn session_view(self: &Arc<Self>, token: &str) -> Result<AccountView> {
        let name = self.session_account(token).await?;
        self.account_view(&name).await
    }
    pub async fn open(raw_store: Arc<dyn unbill_storage::LedgerStore>) -> Result<Arc<Self>> {
        // Init methods run on the raw store before wrapping in StoreServer.
        raw_store.create_secret_key().await?;
        let device_id = raw_store.get_device_id().await?;
        let key = raw_store.get_secret_key().await?;
        let endpoint = UnbillEndpoint::bind(&key).await?;

        // Wrap in StoreServer to serialize all subsequent access.
        let server = Arc::new(StoreServer::spawn(raw_store));

        let (ble, _) = watch::channel(endpoint.ble_snapshot());
        let device = Arc::new(Self {
            server,
            device_id,
            endpoint,
            ble,
        });
        let weak = Arc::downgrade(&device);
        let mut ledger_events = device.server.subscribe();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_millis(500));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            let mut known = Vec::new();
            let mut refresh_known = true;
            loop {
                tokio::select! {
                    _ = interval.tick() => {},
                    event = ledger_events.recv() => {
                        match event {
                            Ok(ServiceEvent::LedgerUpdated { .. })
                            | Err(broadcast::error::RecvError::Lagged(_)) => refresh_known = true,
                            Err(broadcast::error::RecvError::Closed) => break,
                            _ => {},
                        }
                        continue;
                    }
                }
                let Some(device) = weak.upgrade() else { break };
                if refresh_known {
                    match device.collect_peers().await {
                        Ok(peers) => {
                            known = peers;
                            refresh_known = false;
                        }
                        Err(error) => warn!(%error, "could not refresh known BLE peers"),
                    }
                }
                let mut snapshot = device.endpoint.ble_snapshot();
                snapshot.discovered_node_ids = device
                    .endpoint
                    .ble_discovered_node_ids(&known, &snapshot.peers);
                let closed = matches!(snapshot.state, unbill_model::BleState::Closed);
                device.publish_ble(snapshot);
                if closed {
                    break;
                }
            }
        });
        Ok(device)
    }

    pub fn device_id(&self) -> NodeId {
        self.device_id.clone()
    }

    pub fn ble_snapshot(&self) -> unbill_model::BleSnapshot {
        self.ble.borrow().clone()
    }

    /// Current BLE state followed by changes, sampled once per 500 ms.
    pub fn subscribe_ble(&self) -> watch::Receiver<unbill_model::BleSnapshot> {
        self.ble.subscribe()
    }

    fn publish_ble(&self, snapshot: unbill_model::BleSnapshot) {
        self.ble.send_if_modified(|current| {
            if matches!(current.state, unbill_model::BleState::Closed) || *current == snapshot {
                return false;
            }
            *current = snapshot;
            true
        });
    }

    pub async fn close(&self) {
        self.endpoint.close().await;
        self.publish_ble(self.endpoint.ble_snapshot());
    }

    pub fn subscribe(&self) -> broadcast::Receiver<ServiceEvent> {
        self.server.subscribe()
    }

    /// Join with this device's shared name in the target ledger.
    async fn join_ledger(&self, url: &str, label: unbill_model::DeviceLabel) -> Result<()> {
        let (ledger_id, host, token) = parse_join_url(url)?;
        let mut conn = self.endpoint.connect_bi_join(host.clone()).await?;

        let request = JoinRequest {
            token,
            label,
            ledger_id: ledger_id.clone(),
        };
        write_msg(&mut conn.send, &request).await?;

        let reply: JoinReply = read_msg(&mut conn.recv).await?;
        conn.handle.close();

        match reply {
            JoinReply::Ok(response) => {
                let returned = unbill_model::LedgerDoc::from_bytes(&response.ledger_bytes)?;
                if returned.get_ledger()?.ledger_id.to_string() != ledger_id {
                    return Err(UnbillError::InvalidInvitation);
                }
                self.server
                    .persist_joined_ledger(response.ledger_bytes)
                    .await
            }
            JoinReply::Err(e) => Err(UnbillError::Network(format!(
                "join rejected by host: {}",
                e.reason
            ))),
        }
    }

    /// Collect all unique peer NodeIds across all ledgers, excluding this device.
    pub async fn collect_peers(&self) -> Result<Vec<NodeId>> {
        self.server.collect_peers(self.device_id.clone()).await
    }

    /// Dial `peer` and run the full sync exchange for all shared ledgers.
    pub async fn trigger_peer_sync(&self, peer: NodeId) -> Result<()> {
        let mut conn = self.endpoint.connect_bi_sync(peer.clone()).await?;
        let result = async {
            let changed =
                run_sync_session(true, peer, &self.server, &mut conn.recv, &mut conn.send).await?;
            for (id, doc) in changed {
                self.server.merge_and_save_ledger(&id, doc).await?;
            }
            finish_sync_session(&mut conn.recv, &mut conn.send).await
        }
        .await;
        conn.handle.close();
        result
    }

    /// Accept sync/join connections immediately, including offline LAN peers.
    /// Relay registration proceeds independently of incoming connections.
    pub async fn accept_loop(&self) -> Result<()> {
        println!("listening on: {}", self.endpoint.node_id());

        while let Some(accepted) = self.endpoint.accept_bi().await {
            let server = Arc::clone(&self.server);
            let unbill_symmetric_channel::AcceptedBi {
                peer,
                alpn,
                mut send,
                mut recv,
                handle,
            } = accepted;

            tokio::spawn(async move {
                let result: Result<()> = async {
                    match alpn.as_slice() {
                        ALPN_SYNC => {
                            let changed =
                                run_sync_session(false, peer, &server, &mut recv, &mut send)
                                    .await?;
                            for (id, doc) in changed {
                                server.merge_and_save_ledger(&id, doc).await?;
                            }
                            send.shutdown().await.map_err(|error| {
                                UnbillError::Network(format!("finish received sync: {error}"))
                            })?;
                            Ok(())
                        }
                        ALPN_JOIN => handle_join_host(peer, &server, recv, send).await,
                        other => Err(UnbillError::Network(format!(
                            "unknown ALPN from {peer}: {:?}",
                            String::from_utf8_lossy(other)
                        ))),
                    }
                }
                .await;
                if let Err(e) = &result {
                    warn!("connection handler error: {e:#}");
                }
                handle.wait_for_close().await;
            });
        }
        Ok(())
    }
}
// sirno:witness:unbill-device:end

impl AccountView {
    pub fn account_name(&self) -> &str {
        &self.account_name
    }

    pub async fn list_ledgers(&self) -> Result<Vec<LedgerMeta>> {
        self.device
            .server
            .list_account_ledger_metas(&self.account_name)
            .await
            .map_err(account_storage_error)
    }

    pub async fn save_ledger_meta(&self, meta: &LedgerMeta) -> Result<()> {
        self.device
            .server
            .save_account_ledger_meta(&self.account_name, meta)
            .await
            .map_err(account_storage_error)
    }

    pub async fn set_ledger_archived(&self, id: LedgerId, archived: bool) -> Result<()> {
        self.device
            .server
            .set_account_ledger_archived(&self.account_name, id, archived)
            .await
            .map_err(account_storage_error)
    }

    pub async fn asym_sync(&self, id: LedgerId, bytes: Vec<u8>) -> Result<Option<Vec<u8>>> {
        self.device
            .server
            .account_asym_sync(&self.account_name, id, bytes)
            .await
    }

    pub async fn create_invitation(&self, id: LedgerId) -> Result<String> {
        self.device
            .server
            .account_create_invitation(&self.account_name, id, self.device.device_id())
            .await
    }

    pub async fn join_ledger(&self, url: &str, label: unbill_model::DeviceLabel) -> Result<()> {
        if self
            .device
            .server
            .get_account(&self.account_name)
            .await?
            .is_none()
        {
            return Err(UnbillError::NotAuthorized);
        }
        let (id, _, _) = parse_join_url(url)?;
        let id = LedgerId::from_string(&id)
            .map_err(|e| UnbillError::InvalidUrl(format!("invalid ledger ID: {e}")))?;
        self.device.join_ledger(url, label).await?;
        self.device
            .server
            .grant_ledger(&self.account_name, id)
            .await?;
        Ok(())
    }

    pub async fn share_ledger(&self, id: LedgerId, target_name: &str) -> Result<()> {
        match self
            .device
            .server
            .share_account_ledger(&self.account_name, target_name, id)
            .await
        {
            Err(StorageError::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => {
                Err(UnbillError::UserNotFound(target_name.to_owned()))
            }
            other => other.map_err(account_storage_error),
        }
    }

    pub async fn list_accounts(&self) -> Result<Vec<String>> {
        if self
            .device
            .server
            .get_account(&self.account_name)
            .await?
            .is_none()
        {
            return Err(UnbillError::NotAuthorized);
        }
        Ok(self.device.server.list_accounts().await?)
    }

    pub fn subscribe(&self) -> broadcast::Receiver<ServiceEvent> {
        let (tx, rx) = broadcast::channel(256);
        let mut source = self.device.subscribe();
        let server = Arc::clone(&self.device.server);
        let name = self.account_name.clone();
        tokio::spawn(async move {
            loop {
                let event = match source.recv().await {
                    Ok(event) => event,
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(broadcast::error::RecvError::Closed) => break,
                };
                if let ServiceEvent::LedgerUpdated { ledger_id } = &event {
                    let Ok(id) = LedgerId::from_string(ledger_id) else {
                        continue;
                    };
                    if !server.account_has_ledger(&name, id).await.unwrap_or(false) {
                        continue;
                    }
                }
                if tx.send(event).is_err() {
                    break;
                }
            }
        });
        rx
    }
}

fn account_storage_error(error: StorageError) -> UnbillError {
    match error {
        StorageError::Unauthorized => UnbillError::NotAuthorized,
        other => UnbillError::Storage(other),
    }
}

/// Handle an incoming join request using atomic store operations.
async fn handle_join_host<R, W>(
    peer_node_id: NodeId,
    server: &StoreServer,
    mut reader: R,
    mut writer: W,
) -> Result<()>
where
    R: tokio::io::AsyncRead + Unpin,
    W: tokio::io::AsyncWrite + Unpin,
{
    let req: JoinRequest = read_msg(&mut reader).await?;

    // Atomically consume the invitation token.
    let invitation = server.consume_invitation(&req.token).await?;

    let invitation = match invitation {
        None => {
            write_msg(
                &mut writer,
                &JoinReply::Err(JoinError {
                    reason: "unknown or expired token".to_string(),
                }),
            )
            .await?;
            return Ok(());
        }
        Some(inv) => inv,
    };

    if Timestamp::now()? > invitation.expires_at {
        write_msg(
            &mut writer,
            &JoinReply::Err(JoinError {
                reason: "token expired".to_string(),
            }),
        )
        .await?;
        return Ok(());
    }

    if req.ledger_id != invitation.ledger_id.to_string() {
        write_msg(
            &mut writer,
            &JoinReply::Err(JoinError {
                reason: "ledger ID mismatch".to_string(),
            }),
        )
        .await?;
        return Ok(());
    }

    // Atomically add the device to the ledger and get the snapshot.
    let snapshot = server
        .add_device_to_ledger(
            &req.ledger_id,
            unbill_model::NewDevice {
                node_id: peer_node_id,
                label: req.label,
            },
        )
        .await?;

    match snapshot {
        Some(ledger_bytes) => {
            write_msg(&mut writer, &JoinReply::Ok(JoinResponse { ledger_bytes })).await?;
        }
        None => {
            write_msg(
                &mut writer,
                &JoinReply::Err(JoinError {
                    reason: "ledger not found on host".to_string(),
                }),
            )
            .await?;
        }
    }
    Ok(())
}

// sirno:witness:unbill-device:begin
fn parse_join_url(url: &str) -> Result<(String, NodeId, String)> {
    let path = url
        .strip_prefix("unbill://join/")
        .ok_or_else(|| UnbillError::InvalidUrl(format!("invalid join URL: {url:?}")))?;
    let parts: Vec<&str> = path.splitn(3, '/').collect();
    let [ledger_id, host, token] = parts.as_slice() else {
        return Err(UnbillError::InvalidUrl(format!(
            "invalid join URL (expected ledger_id/host_node_id/token): {url:?}"
        )));
    };
    let host = host
        .parse::<NodeId>()
        .map_err(|e| UnbillError::InvalidUrl(format!("invalid host node ID in URL: {e}")))?;
    Ok((ledger_id.to_string(), host, token.to_string()))
}
// sirno:witness:unbill-device:end

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use unbill_model::{Currency, LedgerDoc, LedgerId, NewDevice, NodeId, Timestamp};
    use unbill_storage::LedgerStore;
    use unbill_store_memory::InMemoryStore;

    use super::StoreServer;

    #[tokio::test]
    async fn accounts_isolate_ledgers_and_sessions_survive_view_recreation() {
        let device = super::UnbillDevice::open(Arc::new(InMemoryStore::default()))
            .await
            .unwrap();
        device.register("alice", "first password").await.unwrap();
        device.register("bob", "second password").await.unwrap();
        assert!(device.login("alice", "wrong").await.is_err());
        let alice_token = device.login("alice", "first password").await.unwrap();
        let bob_token = device.login("bob", "second password").await.unwrap();
        let alice = device.session_view(&alice_token).await.unwrap();
        let bob = device.session_view(&bob_token).await.unwrap();
        let ledger_id = LedgerId::new();
        let now = Timestamp::now().unwrap();
        alice
            .save_ledger_meta(&unbill_model::LedgerMeta {
                archived: false,
                ledger_id,
                name: "Private".into(),
                currency: Currency::from_code("USD").unwrap(),
                created_at: now,
                updated_at: now,
            })
            .await
            .unwrap();
        assert_eq!(alice.list_ledgers().await.unwrap().len(), 1);
        assert!(bob.list_ledgers().await.unwrap().is_empty());
        assert!(matches!(
            bob.create_invitation(ledger_id).await,
            Err(unbill_model::UnbillError::NotAuthorized)
        ));
        alice.share_ledger(ledger_id, "bob").await.unwrap();
        assert_eq!(bob.list_ledgers().await.unwrap().len(), 1);
        device
            .revoke_account_ledger("bob", ledger_id)
            .await
            .unwrap();
        assert!(bob.list_ledgers().await.unwrap().is_empty());
        assert_eq!(device.session_account(&alice_token).await.unwrap(), "alice");
        device
            .change_password(&alice_token, "first password", "new password")
            .await
            .unwrap();
        assert!(device.session_view(&alice_token).await.is_err());
        assert!(device.login("alice", "first password").await.is_err());
        assert!(device.login("alice", "new password").await.is_ok());
    }

    #[cfg(feature = "test-helpers")]
    #[tokio::test]
    async fn ble_subscription_suppresses_unchanged_polls_and_reports_close() {
        let device = super::UnbillDevice::open(Arc::new(InMemoryStore::default()))
            .await
            .unwrap();
        let mut rx = device.subscribe_ble();
        assert_eq!(
            rx.borrow_and_update().state,
            unbill_model::BleState::Disabled
        );
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(600), rx.changed())
                .await
                .is_err()
        );
        device.close().await;
        rx.changed().await.unwrap();
        assert_eq!(rx.borrow_and_update().state, unbill_model::BleState::Closed);
        drop(device);
        tokio::time::timeout(std::time::Duration::from_secs(2), rx.changed())
            .await
            .unwrap()
            .unwrap_err();
    }

    fn usd() -> Currency {
        Currency::from_code("USD").unwrap()
    }

    #[tokio::test]
    async fn test_collect_peers_returns_unique_peers_excluding_self()
    -> Result<(), Box<dyn std::error::Error>> {
        let store: Arc<dyn LedgerStore> = Arc::new(InMemoryStore::default());
        let server = StoreServer::spawn(Arc::clone(&store));
        let self_id = NodeId::from_seed(1);
        let peer_a = NodeId::from_seed(2);
        let peer_b = NodeId::from_seed(3);

        // Ledger 1: self + peer_a
        let mut doc1 =
            LedgerDoc::new(LedgerId::new(), "L1".to_string(), usd(), Timestamp::now()?).unwrap();
        doc1.add_device(
            NewDevice {
                node_id: self_id.clone(),
                label: unbill_model::DeviceLabel::new("Unnamed device".into()).unwrap(),
            },
            Timestamp::now()?,
        )
        .unwrap();
        doc1.add_device(
            NewDevice {
                node_id: peer_a.clone(),
                label: unbill_model::DeviceLabel::new("Unnamed device".into()).unwrap(),
            },
            Timestamp::now()?,
        )
        .unwrap();
        let meta1 = unbill_model::LedgerMeta {
            archived: false,
            ledger_id: doc1.get_ledger().unwrap().ledger_id,
            name: "L1".to_string(),
            currency: usd(),
            created_at: Timestamp::now()?,
            updated_at: Timestamp::now()?,
        };
        server.save_ledger_meta(&meta1).await.unwrap();
        server
            .save_ledger(&meta1.ledger_id.to_string(), &mut doc1)
            .await
            .unwrap();

        // Ledger 2: self + peer_a + peer_b
        let mut doc2 =
            LedgerDoc::new(LedgerId::new(), "L2".to_string(), usd(), Timestamp::now()?).unwrap();
        doc2.add_device(
            NewDevice {
                node_id: self_id.clone(),
                label: unbill_model::DeviceLabel::new("Unnamed device".into()).unwrap(),
            },
            Timestamp::now()?,
        )
        .unwrap();
        doc2.add_device(
            NewDevice {
                node_id: peer_a.clone(),
                label: unbill_model::DeviceLabel::new("Unnamed device".into()).unwrap(),
            },
            Timestamp::now()?,
        )
        .unwrap();
        doc2.add_device(
            NewDevice {
                node_id: peer_b.clone(),
                label: unbill_model::DeviceLabel::new("Unnamed device".into()).unwrap(),
            },
            Timestamp::now()?,
        )
        .unwrap();
        let meta2 = unbill_model::LedgerMeta {
            archived: false,
            ledger_id: doc2.get_ledger().unwrap().ledger_id,
            name: "L2".to_string(),
            currency: usd(),
            created_at: Timestamp::now()?,
            updated_at: Timestamp::now()?,
        };
        server.save_ledger_meta(&meta2).await.unwrap();
        server
            .save_ledger(&meta2.ledger_id.to_string(), &mut doc2)
            .await
            .unwrap();

        let peers = server.collect_peers(self_id.clone()).await.unwrap();

        assert_eq!(peers.len(), 2);
        assert!(peers.contains(&peer_a));
        assert!(peers.contains(&peer_b));
        assert!(!peers.contains(&self_id));
        Ok(())
    }

    #[tokio::test]
    async fn test_collect_peers_empty_when_no_ledgers() {
        let store: Arc<dyn LedgerStore> = Arc::new(InMemoryStore::default());
        let server = StoreServer::spawn(store);
        let self_id = NodeId::from_seed(1);

        let peers = server.collect_peers(self_id).await.unwrap();
        assert!(peers.is_empty());
    }
}
