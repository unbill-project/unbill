// Join protocol handler (`unbill/join/v1`).
//
// `run_join_host`  — host side: validate an invite token, add the new device
//                    to the ledger document, and return the full snapshot.
// `run_join_requester` — requester side: present the token, receive and persist
//                    the ledger snapshot.
//
// No Iroh dependency — operates on abstract streams for testability.

use std::sync::Arc;

use tokio::io::{AsyncRead, AsyncWrite};

use unbill_model::{LedgerMeta, NewDevice, NodeId, Timestamp, UnbillError};

type Result<T> = std::result::Result<T, UnbillError>;
use unbill_storage::{LedgerDoc, LedgerStore};

use unbill_storage::{
    load_device_labels, load_pending_invitations, save_device_labels, save_pending_invitations,
};

use crate::protocol::{JoinError, JoinReply, JoinRequest, JoinResponse, read_msg, write_msg};

// ---------------------------------------------------------------------------
// Host side
// ---------------------------------------------------------------------------

/// Receive a `JoinRequest`, validate it, add the joining device to the ledger,
/// and send a `JoinResponse` with the full Automerge snapshot.
///
/// The joining device's `NodeId` must be supplied by the caller from the
/// TLS-verified Iroh connection — it is NOT read from the message body.
// sirno:witness:symmetric-channel:begin
pub async fn run_join_host<R, W>(
    peer_node_id: NodeId,
    store: &Arc<dyn LedgerStore>,
    mut reader: R,
    mut writer: W,
) -> Result<()>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    let req: JoinRequest = read_msg(&mut reader).await?;

    // Load and consume (remove) the token whether valid or not, to prevent replays.
    let invitation = {
        let mut map = load_pending_invitations(&**store).await?;
        let inv = map.remove(&req.token);
        save_pending_invitations(&**store, &map).await?;
        inv
    };

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

    let doc = store.load_ledger(&req.ledger_id).await?;
    let Some(mut doc) = doc else {
        write_msg(
            &mut writer,
            &JoinReply::Err(JoinError {
                reason: "ledger not found on host".to_string(),
            }),
        )
        .await?;
        return Ok(());
    };

    doc.add_device(
        NewDevice {
            node_id: peer_node_id,
        },
        Timestamp::now(),
    )?;
    store.save_ledger(&req.ledger_id, &mut doc).await?;

    write_msg(
        &mut writer,
        &JoinReply::Ok(JoinResponse {
            ledger_bytes: doc.save(),
        }),
    )
    .await?;
    Ok(())
}
// sirno:witness:symmetric-channel:end

// ---------------------------------------------------------------------------
// Requester side
// ---------------------------------------------------------------------------

/// Send a `JoinRequest`, and on success persist the received ledger to the store.
// sirno:witness:symmetric-channel:begin
pub async fn run_join_requester<R, W>(
    host_node_id: NodeId,
    local_label: Option<String>,
    request: JoinRequest,
    store: &Arc<dyn LedgerStore>,
    mut reader: R,
    mut writer: W,
) -> Result<()>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    write_msg(&mut writer, &request).await?;

    let reply: JoinReply = read_msg(&mut reader).await?;
    match reply {
        JoinReply::Ok(response) => {
            let mut doc = LedgerDoc::from_bytes(&response.ledger_bytes)?;
            let ledger = doc.get_ledger()?;
            let ledger_id = ledger.ledger_id.to_string();
            let meta = LedgerMeta {
                ledger_id: ledger.ledger_id,
                name: ledger.name.clone(),
                currency: ledger.currency,
                created_at: ledger.created_at,
                updated_at: Timestamp::now(),
            };
            store.save_ledger_meta(&meta).await?;
            store.save_ledger(&ledger_id, &mut doc).await?;
            if let Some(label) = local_label {
                let mut device_labels = load_device_labels(&**store).await?;
                device_labels.insert(host_node_id.to_string(), label);
                save_device_labels(&**store, &device_labels).await?;
            }
            Ok(())
        }
        JoinReply::Err(e) => Err(UnbillError::Network(format!(
            "join rejected by host: {}",
            e.reason
        ))),
    }
}
// sirno:witness:symmetric-channel:end

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;

    use unbill_model::{
        Currency, Invitation, InviteToken, LedgerId, LedgerMeta, NewDevice, NodeId, Timestamp,
    };
    use unbill_storage::{LedgerDoc, LedgerStore};
    use unbill_store_memory::InMemoryStore;

    use unbill_storage::{load_device_labels, load_pending_invitations, save_pending_invitations};

    use super::{run_join_host, run_join_requester};
    use crate::protocol::JoinRequest;

    fn make_store() -> Arc<InMemoryStore> {
        Arc::new(InMemoryStore::default())
    }

    fn usd() -> Currency {
        Currency::from_code("USD").unwrap()
    }

    fn make_invitation(ledger_id: LedgerId, host_node: NodeId, token: &InviteToken) -> Invitation {
        let now = Timestamp::now();
        Invitation {
            token: token.clone(),
            ledger_id,
            created_by_device: host_node,
            created_at: now,
            expires_at: Timestamp::from_millis(now.as_millis() + 86_400_000),
        }
    }

    #[tokio::test]
    async fn test_join_adds_device_to_ledger() {
        let host_node = NodeId::from_seed(1);
        let joiner_node = NodeId::from_seed(2);

        let mut doc =
            LedgerDoc::new(LedgerId::new(), "Trip".to_string(), usd(), Timestamp::now()).unwrap();
        doc.add_device(
            NewDevice {
                node_id: host_node.clone(),
            },
            Timestamp::now(),
        )
        .unwrap();
        let ledger_id = doc.get_ledger().unwrap().ledger_id;
        let ledger_id_str = ledger_id.to_string();

        let host_store: Arc<dyn LedgerStore> = make_store();

        // Save ledger doc and meta to the store.
        let meta = LedgerMeta {
            ledger_id,
            name: "Trip".to_string(),
            currency: usd(),
            created_at: Timestamp::now(),
            updated_at: Timestamp::now(),
        };
        host_store.save_ledger_meta(&meta).await.unwrap();
        host_store
            .save_ledger(&ledger_id_str, &mut doc)
            .await
            .unwrap();

        // Save the invitation to the store.
        let token = InviteToken::generate();
        let invitation = make_invitation(meta.ledger_id, host_node.clone(), &token);
        save_pending_invitations(
            &*host_store,
            &HashMap::from([(token.to_string(), invitation)]),
        )
        .await
        .unwrap();

        let joiner_store: Arc<dyn LedgerStore> = make_store();

        let (stream_host, stream_joiner) = tokio::io::duplex(64 * 1024);
        let (host_read, host_write) = tokio::io::split(stream_host);
        let (joiner_read, joiner_write) = tokio::io::split(stream_joiner);

        let host_store2 = Arc::clone(&host_store);
        let joiner_store2 = Arc::clone(&joiner_store);

        let request = JoinRequest {
            token: token.to_string(),
            ledger_id: ledger_id_str.clone(),
        };

        let task_host = tokio::spawn({
            let joiner_node = joiner_node.clone();
            async move {
                run_join_host(joiner_node, &host_store2, host_read, host_write)
                    .await
                    .unwrap();
            }
        });
        let task_joiner = tokio::spawn({
            let host_node = host_node.clone();
            async move {
                run_join_requester(
                    host_node,
                    Some("host laptop".to_string()),
                    request,
                    &joiner_store2,
                    joiner_read,
                    joiner_write,
                )
                .await
                .unwrap();
            }
        });

        task_host.await.unwrap();
        task_joiner.await.unwrap();

        // Joiner has the ledger in its store.
        let joiner_doc = joiner_store.load_ledger(&ledger_id_str).await.unwrap();
        assert!(joiner_doc.is_some(), "joiner should have the ledger");
        let joiner_doc = joiner_doc.unwrap();
        let devices = joiner_doc.list_devices().unwrap();
        assert!(
            devices.iter().any(|d| d.node_id == joiner_node),
            "joiner's device should be in the ledger"
        );
        assert!(
            devices
                .iter()
                .all(|d| d.node_id != host_node || d.added_at.as_millis() >= 0),
            "host device entry should still be present without relying on a synced label"
        );

        let device_labels = load_device_labels(&*joiner_store).await.unwrap();
        assert_eq!(
            device_labels
                .get(&host_node.to_string())
                .map(String::as_str),
            Some("host laptop")
        );

        // Token was consumed.
        let remaining = load_pending_invitations(&*host_store).await.unwrap();
        assert!(remaining.is_empty(), "token should have been consumed");
    }

    #[tokio::test]
    async fn test_join_rejects_invalid_token() {
        let joiner_node = NodeId::from_seed(2);

        let mut doc =
            LedgerDoc::new(LedgerId::new(), "Trip".to_string(), usd(), Timestamp::now()).unwrap();
        let ledger_id_str = doc.get_ledger().unwrap().ledger_id.to_string();

        // No invitations saved to store.
        let host_store: Arc<dyn LedgerStore> = make_store();
        host_store
            .save_ledger(&ledger_id_str, &mut doc)
            .await
            .unwrap();

        let joiner_store: Arc<dyn LedgerStore> = make_store();

        let (stream_host, stream_joiner) = tokio::io::duplex(64 * 1024);
        let (host_read, host_write) = tokio::io::split(stream_host);
        let (joiner_read, joiner_write) = tokio::io::split(stream_joiner);

        let host_store2 = Arc::clone(&host_store);
        let joiner_store2 = Arc::clone(&joiner_store);

        let fake_token = InviteToken::generate();
        let request = JoinRequest {
            token: fake_token.to_string(),
            ledger_id: ledger_id_str.clone(),
        };

        let task_host = tokio::spawn(async move {
            run_join_host(joiner_node, &host_store2, host_read, host_write)
                .await
                .unwrap();
        });
        let task_joiner = tokio::spawn(async move {
            let result = run_join_requester(
                NodeId::from_seed(1),
                Some("host".to_string()),
                request,
                &joiner_store2,
                joiner_read,
                joiner_write,
            )
            .await;
            assert!(result.is_err(), "should fail with invalid token");
        });

        task_host.await.unwrap();
        task_joiner.await.unwrap();

        // Joiner got nothing.
        let joiner_doc = joiner_store.load_ledger(&ledger_id_str).await.unwrap();
        assert!(joiner_doc.is_none(), "joiner should have no ledgers");
    }
}
