// Join protocol handler (`unbill/join/v1`).
//
// `run_join_host`  — host side: validate an invite token, add the new device
//                    to the ledger document, and return the full snapshot.
// `run_join_requester` — requester side: present the token, receive and persist
//                    the ledger snapshot.
//
// No Iroh dependency — operates on abstract streams for testability.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use dashmap::DashMap;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::{broadcast, Mutex as TokioMutex};

use crate::doc::LedgerDoc;
use crate::model::{Invitation, LedgerMeta, NewDevice, NodeId, Timestamp};
use crate::service::ServiceEvent;
use crate::storage::LedgerStore;

use super::protocol::{read_msg, write_msg, JoinError, JoinReply, JoinRequest, JoinResponse};

/// Shared map of in-flight join invitations: token hex → `Invitation`.
pub type PendingInvitations = Arc<Mutex<HashMap<String, Invitation>>>;

// ---------------------------------------------------------------------------
// Host side
// ---------------------------------------------------------------------------

/// Receive a `JoinRequest`, validate it, add the joining device to the ledger,
/// and send a `JoinResponse` with the full Automerge snapshot.
///
/// The joining device's `NodeId` must be supplied by the caller from the
/// TLS-verified Iroh connection — it is NOT read from the message body.
pub async fn run_join_host<R, W>(
    peer_node_id: NodeId,
    invitations: &PendingInvitations,
    ledgers: &DashMap<String, Arc<TokioMutex<LedgerDoc>>>,
    store: &Arc<dyn LedgerStore>,
    events: &broadcast::Sender<ServiceEvent>,
    mut reader: R,
    mut writer: W,
) -> anyhow::Result<()>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    let req: JoinRequest = read_msg(&mut reader).await?;

    // Consume (remove) the token whether valid or not, to prevent replays.
    let invitation = {
        let mut map = invitations.lock().unwrap();
        map.remove(&req.token)
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

    let doc_lock = match ledgers.get(&req.ledger_id) {
        Some(l) => Arc::clone(&*l),
        None => {
            write_msg(
                &mut writer,
                &JoinReply::Err(JoinError {
                    reason: "ledger not found on host".to_string(),
                }),
            )
            .await?;
            return Ok(());
        }
    };

    let ledger_bytes = {
        let mut doc = doc_lock.lock().await;
        doc.add_device(
            NewDevice {
                node_id: peer_node_id,
                label: req.label,
            },
            Timestamp::now(),
        )?;
        doc.save()
    };

    store
        .save_ledger_bytes(&req.ledger_id, &ledger_bytes)
        .await?;
    let _ = events.send(ServiceEvent::LedgerUpdated {
        ledger_id: req.ledger_id,
    });

    write_msg(&mut writer, &JoinReply::Ok(JoinResponse { ledger_bytes })).await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Requester side
// ---------------------------------------------------------------------------

/// Send a `JoinRequest`, and on success load and persist the received ledger.
pub async fn run_join_requester<R, W>(
    request: JoinRequest,
    ledgers: &DashMap<String, Arc<TokioMutex<LedgerDoc>>>,
    store: &Arc<dyn LedgerStore>,
    events: &broadcast::Sender<ServiceEvent>,
    mut reader: R,
    mut writer: W,
) -> anyhow::Result<()>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    write_msg(&mut writer, &request).await?;

    let reply: JoinReply = read_msg(&mut reader).await?;
    match reply {
        JoinReply::Ok(response) => {
            let doc = LedgerDoc::from_bytes(&response.ledger_bytes)?;
            let ledger = doc.get_ledger()?;
            let ledger_id = ledger.ledger_id.to_string();
            let meta = LedgerMeta {
                ledger_id: ledger.ledger_id,
                name: ledger.name.clone(),
                currency: ledger.currency.clone(),
                created_at: ledger.created_at,
                updated_at: Timestamp::now(),
            };
            store.save_ledger_meta(&meta).await?;
            store
                .save_ledger_bytes(&ledger_id, &response.ledger_bytes)
                .await?;
            ledgers.insert(ledger_id.clone(), Arc::new(TokioMutex::new(doc)));
            let _ = events.send(ServiceEvent::LedgerUpdated { ledger_id });
            Ok(())
        }
        JoinReply::Err(e) => {
            anyhow::bail!("join rejected by host: {}", e.reason)
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    use dashmap::DashMap;
    use tokio::sync::{broadcast, Mutex as TokioMutex};

    use crate::doc::LedgerDoc;
    use crate::model::{
        Currency, Invitation, InviteToken, LedgerMeta, NewDevice, NodeId, Timestamp, Ulid,
    };
    use crate::service::ServiceEvent;
    use crate::storage::InMemoryStore;

    use super::super::protocol::JoinRequest;
    use super::{run_join_host, run_join_requester, PendingInvitations};

    fn make_store() -> Arc<InMemoryStore> {
        Arc::new(InMemoryStore::default())
    }

    fn make_events() -> broadcast::Sender<ServiceEvent> {
        broadcast::channel(16).0
    }

    fn usd() -> Currency {
        Currency::from_code("USD").unwrap()
    }

    fn make_invitation(ledger_id: Ulid, host_node: NodeId, token: &InviteToken) -> Invitation {
        let now = Timestamp::now();
        Invitation {
            token: token.clone(),
            ledger_id,
            created_by_device: host_node,
            created_at: now,
            // expires far in the future
            expires_at: Timestamp::from_millis(now.as_millis() + 86_400_000),
        }
    }

    #[tokio::test]
    async fn test_join_adds_device_to_ledger() {
        let host_node = NodeId::from_seed(1);
        let joiner_node = NodeId::from_seed(2);

        // Host has a ledger authorized only for itself.
        let mut doc =
            LedgerDoc::new(Ulid::new(), "Trip".to_string(), usd(), Timestamp::now()).unwrap();
        doc.add_device(
            NewDevice {
                node_id: host_node,
                label: "host".to_string(),
            },
            Timestamp::now(),
        )
        .unwrap();
        let ledger_id = doc.get_ledger().unwrap().ledger_id;
        let ledger_id_str = ledger_id.to_string();

        let host_ledgers: DashMap<String, Arc<TokioMutex<LedgerDoc>>> = DashMap::new();
        host_ledgers.insert(ledger_id_str.clone(), Arc::new(TokioMutex::new(doc)));
        let host_store: Arc<dyn crate::storage::LedgerStore> = make_store();

        // Save meta so the host can look it up.
        let meta = LedgerMeta {
            ledger_id,
            name: "Trip".to_string(),
            currency: usd(),
            created_at: Timestamp::now(),
            updated_at: Timestamp::now(),
        };
        host_store.save_ledger_meta(&meta).await.unwrap();

        let token = InviteToken::generate();
        let invitations: PendingInvitations = Arc::new(Mutex::new(HashMap::from([(
            token.to_string(),
            make_invitation(ledger_id, host_node, &token),
        )])));

        let joiner_ledgers: DashMap<String, Arc<TokioMutex<LedgerDoc>>> = DashMap::new();
        let joiner_store: Arc<dyn crate::storage::LedgerStore> = make_store();

        let (stream_host, stream_joiner) = tokio::io::duplex(64 * 1024);
        let (host_read, host_write) = tokio::io::split(stream_host);
        let (joiner_read, joiner_write) = tokio::io::split(stream_joiner);

        let host_store2 = Arc::clone(&host_store);
        let joiner_store2 = Arc::clone(&joiner_store);
        let events_host = make_events();
        let events_joiner = make_events();
        let host_ledgers = Arc::new(host_ledgers);
        let joiner_ledgers = Arc::new(joiner_ledgers);
        let hl2 = Arc::clone(&host_ledgers);
        let jl2 = Arc::clone(&joiner_ledgers);
        let invitations2 = Arc::clone(&invitations);

        let request = JoinRequest {
            token: token.to_string(),
            ledger_id: ledger_id_str.clone(),
            label: "joiner's phone".to_string(),
        };

        let task_host = tokio::spawn(async move {
            run_join_host(
                joiner_node,
                &invitations2,
                &hl2,
                &host_store2,
                &events_host,
                host_read,
                host_write,
            )
            .await
            .unwrap();
        });
        let task_joiner = tokio::spawn(async move {
            run_join_requester(
                request,
                &jl2,
                &joiner_store2,
                &events_joiner,
                joiner_read,
                joiner_write,
            )
            .await
            .unwrap();
        });

        task_host.await.unwrap();
        task_joiner.await.unwrap();

        // Joiner now has the ledger in their map.
        assert!(
            joiner_ledgers.contains_key(&ledger_id_str),
            "joiner should have the ledger"
        );

        // The ledger on both sides should now have joiner's device authorized.
        let joiner_doc = joiner_ledgers.get(&ledger_id_str).unwrap();
        let joiner_doc = joiner_doc.lock().await;
        let devices = joiner_doc.list_devices().unwrap();
        assert!(
            devices.iter().any(|d| d.node_id == joiner_node),
            "joiner's device should be in the ledger"
        );

        // Token was consumed.
        assert!(
            invitations.lock().unwrap().is_empty(),
            "token should have been consumed"
        );
    }

    #[tokio::test]
    async fn test_join_rejects_invalid_token() {
        let host_node = NodeId::from_seed(1);
        let joiner_node = NodeId::from_seed(2);

        let doc = LedgerDoc::new(Ulid::new(), "Trip".to_string(), usd(), Timestamp::now()).unwrap();
        let ledger_id_str = doc.get_ledger().unwrap().ledger_id.to_string();

        let host_ledgers: DashMap<String, Arc<TokioMutex<LedgerDoc>>> = DashMap::new();
        host_ledgers.insert(ledger_id_str.clone(), Arc::new(TokioMutex::new(doc)));
        let host_store: Arc<dyn crate::storage::LedgerStore> = make_store();

        // No invitations at all.
        let invitations: PendingInvitations = Arc::new(Mutex::new(HashMap::new()));

        let joiner_ledgers: DashMap<String, Arc<TokioMutex<LedgerDoc>>> = DashMap::new();
        let joiner_store: Arc<dyn crate::storage::LedgerStore> = make_store();

        let (stream_host, stream_joiner) = tokio::io::duplex(64 * 1024);
        let (host_read, host_write) = tokio::io::split(stream_host);
        let (joiner_read, joiner_write) = tokio::io::split(stream_joiner);

        let host_store2 = Arc::clone(&host_store);
        let joiner_store2 = Arc::clone(&joiner_store);
        let events_host = make_events();
        let events_joiner = make_events();
        let hl = Arc::new(host_ledgers);
        let jl = Arc::new(joiner_ledgers);
        let hl2 = Arc::clone(&hl);
        let jl2 = Arc::clone(&jl);

        let fake_token = InviteToken::generate();
        let request = JoinRequest {
            token: fake_token.to_string(),
            ledger_id: ledger_id_str,
            label: "joiner".to_string(),
        };

        let task_host = tokio::spawn(async move {
            run_join_host(
                joiner_node,
                &invitations,
                &hl2,
                &host_store2,
                &events_host,
                host_read,
                host_write,
            )
            .await
            .unwrap();
        });
        let task_joiner = tokio::spawn(async move {
            let result = run_join_requester(
                request,
                &jl2,
                &joiner_store2,
                &events_joiner,
                joiner_read,
                joiner_write,
            )
            .await;
            assert!(result.is_err(), "should fail with invalid token");
        });

        task_host.await.unwrap();
        task_joiner.await.unwrap();

        // Joiner got nothing.
        assert!(jl.is_empty(), "joiner should have no ledgers");
    }
}
