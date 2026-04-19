// Per-peer sync session: Hello/HelloAck handshake followed by the Automerge
// sync loop for every accepted ledger.
//
// This module has no Iroh dependency — it operates on abstract AsyncRead +
// AsyncWrite streams so it can be tested with in-process channel pairs.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::broadcast;

use crate::doc::LedgerDoc;
use crate::model::NodeId;
use crate::service::ServiceEvent;
use crate::storage::LedgerStore;

use super::protocol::{read_msg, write_msg, Hello, HelloAck, SyncDone, SyncFrame, SyncMsg};

struct LedgerSyncState {
    sync_state: automerge::sync::State,
    /// We have sent `SyncDone` for this ledger.
    we_done: bool,
    /// Peer has sent `SyncDone` for this ledger.
    peer_done: bool,
}

/// Drive one full sync session over an already-open bidirectional stream.
///
/// * `is_initiator` — the side that sends `Hello` first.
/// * `peer_node_id` — TLS-verified identity of the remote device (passed in
///   by the caller from the Iroh connection context).
/// * `store` — used to list ledgers, load docs for the session, and persist
///   updated bytes after merging remote changes.
/// * `events` — `LedgerUpdated` is emitted for every ledger that received new
///   changes.
///
/// Ledger documents are loaded from the store at the start of the session and
/// held in memory only for its duration. Nothing is cached between sessions.
pub async fn run_sync_session<R, W>(
    is_initiator: bool,
    peer_node_id: NodeId,
    store: &Arc<dyn LedgerStore>,
    events: &broadcast::Sender<ServiceEvent>,
    mut reader: R,
    mut writer: W,
) -> anyhow::Result<()>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    // -----------------------------------------------------------------------
    // Step 1: Hello / HelloAck handshake
    // -----------------------------------------------------------------------

    let accepted: Vec<String> = if is_initiator {
        let metas = store.list_ledgers().await?;
        let my_ids: Vec<String> = metas.iter().map(|m| m.ledger_id.to_string()).collect();
        write_msg(&mut writer, &SyncFrame::Hello(Hello { ledger_ids: my_ids })).await?;
        let frame: SyncFrame = read_msg(&mut reader).await?;
        match frame {
            SyncFrame::HelloAck(ack) => ack.accepted,
            other => anyhow::bail!("expected HelloAck, got {:?}", other),
        }
    } else {
        let frame: SyncFrame = read_msg(&mut reader).await?;
        let hello = match frame {
            SyncFrame::Hello(h) => h,
            other => anyhow::bail!("expected Hello, got {:?}", other),
        };
        let mut accepted = Vec::new();
        let mut rejected = Vec::new();
        for id in &hello.ledger_ids {
            let bytes = store.load_ledger_bytes(id).await.unwrap_or_default();
            if bytes.is_empty() {
                rejected.push(id.clone());
                continue;
            }
            match LedgerDoc::from_bytes(&bytes) {
                Ok(doc) => {
                    if doc.is_device_authorized(&peer_node_id).unwrap_or(false) {
                        accepted.push(id.clone());
                    } else {
                        rejected.push(id.clone());
                    }
                }
                Err(_) => rejected.push(id.clone()),
            }
        }
        write_msg(
            &mut writer,
            &SyncFrame::HelloAck(HelloAck {
                accepted: accepted.clone(),
                rejected,
            }),
        )
        .await?;
        accepted
    };

    if accepted.is_empty() {
        return Ok(());
    }

    // -----------------------------------------------------------------------
    // Step 2: Load accepted ledgers into session-local memory
    // -----------------------------------------------------------------------

    let mut docs: HashMap<String, LedgerDoc> = HashMap::new();
    for id in &accepted {
        let bytes = store.load_ledger_bytes(id).await?;
        let doc = LedgerDoc::from_bytes(&bytes)
            .map_err(|e| anyhow::anyhow!("failed to load ledger {id}: {e}"))?;
        docs.insert(id.clone(), doc);
    }

    // -----------------------------------------------------------------------
    // Step 3: Automerge sync loop
    // -----------------------------------------------------------------------

    let mut states: HashMap<String, LedgerSyncState> = accepted
        .into_iter()
        .map(|id| {
            (
                id,
                LedgerSyncState {
                    sync_state: automerge::sync::State::new(),
                    we_done: false,
                    peer_done: false,
                },
            )
        })
        .collect();

    let mut ledgers_with_remote_changes: Vec<String> = Vec::new();

    loop {
        // --- Send phase ---
        for (id, state) in states.iter_mut() {
            if state.we_done {
                continue;
            }
            let doc = docs
                .get_mut(id)
                .ok_or_else(|| anyhow::anyhow!("ledger disappeared mid-sync: {id}"))?;
            match doc.generate_sync_message(&mut state.sync_state) {
                Some(msg) => {
                    write_msg(
                        &mut writer,
                        &SyncFrame::Msg(SyncMsg {
                            ledger_id: id.clone(),
                            payload: msg.encode(),
                        }),
                    )
                    .await?;
                }
                None => {
                    write_msg(
                        &mut writer,
                        &SyncFrame::Done(SyncDone {
                            ledger_id: id.clone(),
                        }),
                    )
                    .await?;
                    state.we_done = true;
                }
            }
        }

        // --- Check termination ---
        if states.values().all(|s| s.we_done && s.peer_done) {
            break;
        }
        if states.values().all(|s| s.peer_done) {
            break;
        }

        // --- Read one incoming frame ---
        let frame: SyncFrame = read_msg(&mut reader).await?;
        match frame {
            SyncFrame::Msg(m) => {
                let state = states.get_mut(&m.ledger_id).ok_or_else(|| {
                    anyhow::anyhow!("sync msg for unknown ledger: {}", m.ledger_id)
                })?;
                let msg = automerge::sync::Message::decode(&m.payload)
                    .map_err(|e| anyhow::anyhow!("bad sync message bytes: {e}"))?;
                let doc = docs
                    .get_mut(&m.ledger_id)
                    .ok_or_else(|| anyhow::anyhow!("ledger disappeared: {}", m.ledger_id))?;
                doc.receive_sync_message(&mut state.sync_state, msg)?;
                if !ledgers_with_remote_changes.contains(&m.ledger_id) {
                    ledgers_with_remote_changes.push(m.ledger_id);
                }
            }
            SyncFrame::Done(d) => {
                let state = states
                    .get_mut(&d.ledger_id)
                    .ok_or_else(|| anyhow::anyhow!("done for unknown ledger: {}", d.ledger_id))?;
                state.peer_done = true;
            }
            other => {
                anyhow::bail!("unexpected frame during sync loop: {:?}", other);
            }
        }
    }

    // -----------------------------------------------------------------------
    // Step 4: Persist and emit events for ledgers that received remote changes
    // -----------------------------------------------------------------------

    for id in &ledgers_with_remote_changes {
        if let Some(doc) = docs.get_mut(id) {
            let bytes = doc.save();
            store.save_ledger_bytes(id, &bytes).await?;
            let _ = events.send(ServiceEvent::LedgerUpdated {
                ledger_id: id.clone(),
            });
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use tokio::sync::broadcast;

    use crate::doc::LedgerDoc;
    use crate::model::{Currency, NewBill, NewDevice, NodeId, Share, Timestamp, Ulid};
    use crate::service::ServiceEvent;
    use crate::storage::{InMemoryStore, LedgerStore};

    use super::run_sync_session;

    fn make_store() -> Arc<InMemoryStore> {
        Arc::new(InMemoryStore::default())
    }

    fn make_events() -> broadcast::Sender<ServiceEvent> {
        broadcast::channel(16).0
    }

    fn usd() -> Currency {
        Currency::from_code("USD").unwrap()
    }

    /// Save a doc and its meta to a store.
    async fn save_doc(store: &dyn LedgerStore, doc: &mut LedgerDoc) {
        let ledger = doc.get_ledger().unwrap();
        let id = ledger.ledger_id.to_string();
        let meta = crate::model::LedgerMeta {
            ledger_id: ledger.ledger_id,
            name: ledger.name.clone(),
            currency: ledger.currency,
            created_at: ledger.created_at,
            updated_at: Timestamp::now(),
        };
        store.save_ledger_meta(&meta).await.unwrap();
        store.save_ledger_bytes(&id, &doc.save()).await.unwrap();
    }

    /// Run sync between two stores over an in-process duplex channel.
    async fn sync_pair(
        store_a: Arc<dyn LedgerStore>,
        store_b: Arc<dyn LedgerStore>,
        peer_a: NodeId,
        peer_b: NodeId,
    ) {
        let events_a = make_events();
        let events_b = make_events();

        let (stream_a, stream_b) = tokio::io::duplex(64 * 1024);
        let (a_read, a_write) = tokio::io::split(stream_a);
        let (b_read, b_write) = tokio::io::split(stream_b);

        let sa = Arc::clone(&store_a);
        let sb = Arc::clone(&store_b);

        let task_a = tokio::spawn(async move {
            run_sync_session(true, peer_b, &sa, &events_a, a_read, a_write)
                .await
                .unwrap();
        });
        let task_b = tokio::spawn(async move {
            run_sync_session(false, peer_a, &sb, &events_b, b_read, b_write)
                .await
                .unwrap();
        });

        task_a.await.unwrap();
        task_b.await.unwrap();
    }

    #[tokio::test]
    async fn test_sync_empty_hello_ack_no_shared_ledgers() {
        let node_a = NodeId::from_seed(1);
        let node_b = NodeId::from_seed(2);

        // A has a ledger that authorizes B; B has no ledgers.
        let store_a: Arc<dyn LedgerStore> = make_store();
        let store_b: Arc<dyn LedgerStore> = make_store();

        let mut doc_a =
            LedgerDoc::new(Ulid::new(), "Test".to_string(), usd(), Timestamp::now()).unwrap();
        doc_a
            .add_device(
                NewDevice {
                    node_id: node_b,
                    label: "B".to_string(),
                },
                Timestamp::now(),
            )
            .unwrap();
        save_doc(&*store_a, &mut doc_a).await;

        sync_pair(store_a, store_b, node_a, node_b).await;
        // No panic = both sides closed cleanly with empty accepted list.
    }

    #[tokio::test]
    async fn test_sync_converges_after_divergence() {
        let node_a = NodeId::from_seed(1);
        let node_b = NodeId::from_seed(2);

        // Build a base ledger that both A and B start with.
        let mut base =
            LedgerDoc::new(Ulid::new(), "Trip".to_string(), usd(), Timestamp::now()).unwrap();
        base.add_device(
            NewDevice {
                node_id: node_a,
                label: "A".to_string(),
            },
            Timestamp::now(),
        )
        .unwrap();
        base.add_device(
            NewDevice {
                node_id: node_b,
                label: "B".to_string(),
            },
            Timestamp::now(),
        )
        .unwrap();
        let payer = Ulid::from_u128(99);
        base.add_member(
            crate::model::NewMember {
                user_id: payer,
                display_name: "Payer".to_string(),
                added_by: payer,
            },
            Timestamp::now(),
        )
        .unwrap();
        let base_bytes = base.save();
        let ledger_id = base.get_ledger().unwrap().ledger_id.to_string();

        // Fork into two independent docs.
        let mut doc_a = LedgerDoc::from_bytes(&base_bytes).unwrap();
        let mut doc_b = LedgerDoc::from_bytes(&base_bytes).unwrap();

        doc_a
            .add_bill(
                NewBill {
                    payer_user_id: payer,
                    amount_cents: 1000,
                    description: "from A".to_string(),
                    shares: vec![Share {
                        user_id: payer,
                        shares: 1,
                    }],
                    prev: vec![],
                },
                node_a,
                Timestamp::now(),
            )
            .unwrap();

        doc_b
            .add_bill(
                NewBill {
                    payer_user_id: payer,
                    amount_cents: 2000,
                    description: "from B".to_string(),
                    shares: vec![Share {
                        user_id: payer,
                        shares: 1,
                    }],
                    prev: vec![],
                },
                node_b,
                Timestamp::now(),
            )
            .unwrap();

        let store_a: Arc<dyn LedgerStore> = make_store();
        let store_b: Arc<dyn LedgerStore> = make_store();
        save_doc(&*store_a, &mut doc_a).await;
        save_doc(&*store_b, &mut doc_b).await;

        sync_pair(Arc::clone(&store_a), Arc::clone(&store_b), node_a, node_b).await;

        // Load final state from stores.
        let bytes_a = store_a.load_ledger_bytes(&ledger_id).await.unwrap();
        let bytes_b = store_b.load_ledger_bytes(&ledger_id).await.unwrap();
        let doc_a_final = LedgerDoc::from_bytes(&bytes_a).unwrap();
        let doc_b_final = LedgerDoc::from_bytes(&bytes_b).unwrap();

        let bills_a = doc_a_final.list_bills().unwrap();
        let bills_b = doc_b_final.list_bills().unwrap();

        assert_eq!(bills_a.0.len(), 2, "A should have both bills after sync");
        assert_eq!(bills_b.0.len(), 2, "B should have both bills after sync");

        let mut descs_a: Vec<_> = bills_a.iter().map(|b| b.description.clone()).collect();
        let mut descs_b: Vec<_> = bills_b.iter().map(|b| b.description.clone()).collect();
        descs_a.sort();
        descs_b.sort();
        assert_eq!(descs_a, descs_b);
    }

    #[tokio::test]
    async fn test_sync_unauthorized_device_rejected() {
        let node_a = NodeId::from_seed(1);
        let node_b = NodeId::from_seed(2);

        // A's ledger does NOT authorize B.
        let store_a: Arc<dyn LedgerStore> = make_store();
        let store_b: Arc<dyn LedgerStore> = make_store();

        let mut doc_a =
            LedgerDoc::new(Ulid::new(), "Private".to_string(), usd(), Timestamp::now()).unwrap();
        let id = doc_a.get_ledger().unwrap().ledger_id.to_string();
        save_doc(&*store_a, &mut doc_a).await;

        // B has the same ledger ID.
        let mut doc_b =
            LedgerDoc::new(Ulid::new(), "Same id?".to_string(), usd(), Timestamp::now()).unwrap();
        // Manually set the same ID by saving with A's id key.
        store_b.save_ledger_bytes(&id, &doc_b.save()).await.unwrap();

        sync_pair(store_a, store_b, node_a, node_b).await;
        // No panic — A just rejects the ledger.
    }
}
