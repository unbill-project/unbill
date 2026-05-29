// unbill-device: device-side implementation of the AsymChannel contract.
//
// Holds a local LedgerStore and provides invitation management, peer sync,
// and the Automerge sync server-side handler.  Does not know about the
// AsymChannel trait — LocalAsymChannel wraps this and wires up the trait.

use std::sync::Arc;

pub use unbill_event::ServiceEvent;
pub use unbill_storage::StoreServer;

use tokio::sync::broadcast;
use tracing::warn;
use unbill_model::error::{Result, UnbillError};
use unbill_model::{LedgerId, NodeId, Timestamp};
use unbill_symmetric_channel::{
    ALPN_JOIN, ALPN_SYNC, JoinError, JoinReply, JoinRequest, JoinResponse, UnbillEndpoint,
    read_msg, run_sync_session, write_msg,
};

// sirno:witness:unbill-device:begin
pub struct UnbillDevice {
    server: Arc<StoreServer>,
    device_id: NodeId,
    endpoint: UnbillEndpoint,
}

impl UnbillDevice {
    pub async fn open(raw_store: Arc<dyn unbill_storage::LedgerStore>) -> Result<Arc<Self>> {
        // Init methods run on the raw store before wrapping in StoreServer.
        raw_store.create_secret_key().await?;
        let device_id = raw_store.get_device_id().await?;
        let key = raw_store.get_secret_key().await?;
        let endpoint = UnbillEndpoint::bind(&key).await?;

        // Wrap in StoreServer to serialize all subsequent access.
        let server = Arc::new(StoreServer::spawn(raw_store));

        Ok(Arc::new(Self {
            server,
            device_id,
            endpoint,
        }))
    }

    pub fn store(&self) -> &Arc<StoreServer> {
        &self.server
    }

    pub fn device_id(&self) -> NodeId {
        self.device_id.clone()
    }

    pub fn subscribe(&self) -> broadcast::Receiver<ServiceEvent> {
        self.server.subscribe()
    }

    /// Generate a join invite URL for `ledger_id` and persist the pending invitation.
    pub async fn create_invitation(&self, ledger_id: LedgerId) -> Result<String> {
        self.server
            .create_invitation(ledger_id, self.device_id.clone())
            .await
    }

    /// Dial the host in `url` and join the ledger. `label` is an optional
    /// device-local nickname for the host stored after a successful join.
    pub async fn join_ledger(&self, url: &str, label: Option<String>) -> Result<()> {
        let (ledger_id, host, token) = parse_join_url(url)?;
        let mut conn = self.endpoint.connect_bi_join(host.clone()).await?;

        let request = JoinRequest {
            token,
            ledger_id: ledger_id.clone(),
        };
        write_msg(&mut conn.send, &request).await?;

        let reply: JoinReply = read_msg(&mut conn.recv).await?;
        conn.handle.close();

        match reply {
            JoinReply::Ok(response) => {
                self.server
                    .persist_joined_ledger(response.ledger_bytes, host, label)
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
        tracing::debug!(%peer, "trigger_peer_sync: connecting");
        let conn = self.endpoint.connect_bi_sync(peer.clone()).await?;
        let changed = run_sync_session(true, peer, &self.server, conn.recv, conn.send).await?;
        conn.handle.close();
        tracing::debug!(
            changed_count = changed.len(),
            "trigger_peer_sync: session done"
        );
        for (id, doc) in changed {
            tracing::debug!(ledger_id = %id, "trigger_peer_sync: merge_and_save_ledger");
            self.server.merge_and_save_ledger(&id, doc).await?;
        }
        Ok(())
    }

    /// Receive one Automerge sync message, persist any changes, and return the
    /// server's response (or `None` if it has nothing new to send).
    pub async fn asym_sync(&self, ledger_id: LedgerId, bytes: Vec<u8>) -> Result<Option<Vec<u8>>> {
        self.server.asym_sync(ledger_id, bytes).await
    }

    /// Wait for the endpoint to be ready, print the readiness line, then accept
    /// incoming sync/join connections until an error occurs or the endpoint closes.
    pub async fn accept_loop(&self) -> Result<()> {
        self.endpoint.wait_for_ready().await;
        println!("listening on: {}", self.endpoint.node_id());

        while let Some(accepted) = self.endpoint.accept_bi().await {
            let server = Arc::clone(&self.server);
            let unbill_symmetric_channel::AcceptedBi {
                peer,
                alpn,
                send,
                recv,
                handle,
            } = accepted;

            tokio::spawn(async move {
                let result: Result<()> = async {
                    match alpn.as_slice() {
                        ALPN_SYNC => {
                            let changed =
                                run_sync_session(false, peer, &server, recv, send).await?;
                            for (id, doc) in changed {
                                server.merge_and_save_ledger(&id, doc).await?;
                            }
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

    if Timestamp::now() > invitation.expires_at {
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
        .add_device_to_ledger(&req.ledger_id, peer_node_id)
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

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use unbill_model::{Currency, LedgerDoc, LedgerId, NewDevice, NodeId, Timestamp};
    use unbill_storage::LedgerStore;
    use unbill_store_memory::InMemoryStore;

    use super::StoreServer;

    fn usd() -> Currency {
        Currency::from_code("USD").unwrap()
    }

    #[tokio::test]
    async fn test_collect_peers_returns_unique_peers_excluding_self() {
        let store: Arc<dyn LedgerStore> = Arc::new(InMemoryStore::default());
        let server = StoreServer::spawn(Arc::clone(&store));
        let self_id = NodeId::from_seed(1);
        let peer_a = NodeId::from_seed(2);
        let peer_b = NodeId::from_seed(3);

        // Ledger 1: self + peer_a
        let mut doc1 =
            LedgerDoc::new(LedgerId::new(), "L1".to_string(), usd(), Timestamp::now()).unwrap();
        doc1.add_device(
            NewDevice {
                node_id: self_id.clone(),
            },
            Timestamp::now(),
        )
        .unwrap();
        doc1.add_device(
            NewDevice {
                node_id: peer_a.clone(),
            },
            Timestamp::now(),
        )
        .unwrap();
        let meta1 = unbill_model::LedgerMeta {
            ledger_id: doc1.get_ledger().unwrap().ledger_id,
            name: "L1".to_string(),
            currency: usd(),
            created_at: Timestamp::now(),
            updated_at: Timestamp::now(),
        };
        server.save_ledger_meta(&meta1).await.unwrap();
        server
            .save_ledger(&meta1.ledger_id.to_string(), &mut doc1)
            .await
            .unwrap();

        // Ledger 2: self + peer_a + peer_b
        let mut doc2 =
            LedgerDoc::new(LedgerId::new(), "L2".to_string(), usd(), Timestamp::now()).unwrap();
        doc2.add_device(
            NewDevice {
                node_id: self_id.clone(),
            },
            Timestamp::now(),
        )
        .unwrap();
        doc2.add_device(
            NewDevice {
                node_id: peer_a.clone(),
            },
            Timestamp::now(),
        )
        .unwrap();
        doc2.add_device(
            NewDevice {
                node_id: peer_b.clone(),
            },
            Timestamp::now(),
        )
        .unwrap();
        let meta2 = unbill_model::LedgerMeta {
            ledger_id: doc2.get_ledger().unwrap().ledger_id,
            name: "L2".to_string(),
            currency: usd(),
            created_at: Timestamp::now(),
            updated_at: Timestamp::now(),
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
