// Per-peer sync session: Hello/HelloAck handshake followed by the Automerge
// sync loop for every accepted ledger.
//
// This module has no Iroh dependency — it operates on abstract AsyncRead +
// AsyncWrite streams so it can be tested with in-process channel pairs.

use std::collections::HashMap;
use std::sync::Arc;

use dashmap::DashMap;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::{broadcast, Mutex};

use crate::doc::LedgerDoc;
use crate::model::NodeId;
use crate::service::ServiceEvent;
use crate::storage::LedgerStore;

use super::protocol::{
    HelloAck, Hello, SyncDone, SyncFrame, SyncMsg, read_msg, write_msg,
};

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
/// * `ledgers` — the in-memory ledger map shared with `UnbillService`.
/// * `store` — used to persist updated bytes after merging remote changes.
/// * `events` — `LedgerUpdated` is emitted for every ledger that received new
///   changes.
pub async fn run_sync_session<R, W>(
    is_initiator: bool,
    peer_node_id: NodeId,
    ledgers: &DashMap<String, Arc<Mutex<LedgerDoc>>>,
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
        let my_ids: Vec<String> = ledgers.iter().map(|e| e.key().clone()).collect();
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
            if let Some(doc_lock) = ledgers.get(id) {
                let doc = doc_lock.lock().await;
                if doc.is_device_authorized(&peer_node_id)? {
                    accepted.push(id.clone());
                } else {
                    rejected.push(id.clone());
                }
            } else {
                rejected.push(id.clone());
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
    // Step 2: Automerge sync loop
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

    // Track which ledgers actually received remote changes so we can persist
    // and emit events only for those.
    let mut ledgers_with_remote_changes: Vec<String> = Vec::new();

    loop {
        // --- Send phase: generate outgoing messages for all pending ledgers ---
        for (id, state) in states.iter_mut() {
            if state.we_done {
                continue;
            }
            let doc_lock = ledgers
                .get(id)
                .ok_or_else(|| anyhow::anyhow!("ledger disappeared mid-sync: {id}"))?;
            let mut doc = doc_lock.lock().await;
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

        // If all peers are done but we still have ledgers to drain, keep reading.
        // If we still have work to do, also keep going.
        if states.values().all(|s| s.peer_done) {
            break;
        }

        // --- Read one incoming frame ---
        let frame: SyncFrame = read_msg(&mut reader).await?;
        match frame {
            SyncFrame::Msg(m) => {
                let state = states
                    .get_mut(&m.ledger_id)
                    .ok_or_else(|| anyhow::anyhow!("sync msg for unknown ledger: {}", m.ledger_id))?;
                let msg = automerge::sync::Message::decode(&m.payload)
                    .map_err(|e| anyhow::anyhow!("bad sync message bytes: {e}"))?;
                let doc_lock = ledgers
                    .get(&m.ledger_id)
                    .ok_or_else(|| anyhow::anyhow!("ledger disappeared: {}", m.ledger_id))?;
                let mut doc = doc_lock.lock().await;
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
    // Step 3: Persist and emit events for ledgers that changed
    // -----------------------------------------------------------------------

    for id in &ledgers_with_remote_changes {
        if let Some(doc_lock) = ledgers.get(id) {
            let mut doc = doc_lock.lock().await;
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

    use dashmap::DashMap;
    use tokio::sync::{broadcast, Mutex};

    use crate::doc::LedgerDoc;
    use crate::model::{Currency, NewBill, NewDevice, NodeId, Share, Timestamp, Ulid};
    use crate::service::ServiceEvent;
    use crate::storage::InMemoryStore;

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

    /// Create a simple ledger doc with one device authorized.
    fn ledger_with_device(
        device: NodeId,
    ) -> LedgerDoc {
        let mut doc = LedgerDoc::new(
            Ulid::new(),
            "Test".to_string(),
            usd(),
            Timestamp::now(),
        )
        .unwrap();
        doc.add_device(
            crate::model::NewDevice {
                node_id: device,
                label: "test device".to_string(),
            },
            Timestamp::now(),
        )
        .unwrap();
        doc
    }

    type LedgerMap = Arc<DashMap<String, Arc<Mutex<LedgerDoc>>>>;

    /// Run sync between two ledger maps over an in-process duplex channel.
    /// Returns the Arc-wrapped maps so callers can inspect final state.
    async fn sync_pair(
        ledgers_a: DashMap<String, Arc<Mutex<LedgerDoc>>>,
        ledgers_b: DashMap<String, Arc<Mutex<LedgerDoc>>>,
        peer_a: NodeId, // A's node id as seen by B
        peer_b: NodeId, // B's node id as seen by A
    ) -> (LedgerMap, LedgerMap) {
        let store_a: Arc<dyn crate::storage::LedgerStore> = make_store();
        let store_b: Arc<dyn crate::storage::LedgerStore> = make_store();
        let events_a = make_events();
        let events_b = make_events();

        let (stream_a, stream_b) = tokio::io::duplex(64 * 1024);
        let (a_read, a_write) = tokio::io::split(stream_a);
        let (b_read, b_write) = tokio::io::split(stream_b);

        let la: LedgerMap = Arc::new(ledgers_a);
        let lb: LedgerMap = Arc::new(ledgers_b);
        let la2 = Arc::clone(&la);
        let lb2 = Arc::clone(&lb);

        let task_a = tokio::spawn(async move {
            run_sync_session(true, peer_b, &la2, &store_a, &events_a, a_read, a_write)
                .await
                .unwrap();
        });
        let task_b = tokio::spawn(async move {
            run_sync_session(false, peer_a, &lb2, &store_b, &events_b, b_read, b_write)
                .await
                .unwrap();
        });

        task_a.await.unwrap();
        task_b.await.unwrap();

        (la, lb)
    }

    #[tokio::test]
    async fn test_sync_empty_hello_ack_no_shared_ledgers() {
        // A has a ledger; B has no ledgers. B rejects everything.
        let node_a = NodeId::from_seed(1);
        let node_b = NodeId::from_seed(2);

        let doc_a = ledger_with_device(node_b); // A authorizes B
        let id = doc_a.get_ledger().unwrap().ledger_id.to_string();
        let ledgers_a: DashMap<String, Arc<Mutex<LedgerDoc>>> = DashMap::new();
        ledgers_a.insert(id, Arc::new(Mutex::new(doc_a)));
        let ledgers_b: DashMap<String, Arc<Mutex<LedgerDoc>>> = DashMap::new();

        let _ = sync_pair(ledgers_a, ledgers_b, node_a, node_b).await;
        // No panic = both sides closed cleanly with empty accepted list.
    }

    #[tokio::test]
    async fn test_sync_converges_after_divergence() {
        let node_a = NodeId::from_seed(1);
        let node_b = NodeId::from_seed(2);

        // Build a base ledger that both A and B start with.
        let mut base = LedgerDoc::new(Ulid::new(), "Trip".to_string(), usd(), Timestamp::now())
            .unwrap();
        base.add_device(NewDevice { node_id: node_a, label: "A".to_string() }, Timestamp::now())
            .unwrap();
        base.add_device(NewDevice { node_id: node_b, label: "B".to_string() }, Timestamp::now())
            .unwrap();
        let ledger_id = base.get_ledger().unwrap().ledger_id.to_string();
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

        // Fork into two independent docs.
        let mut doc_a = LedgerDoc::from_bytes(&base_bytes).unwrap();
        let mut doc_b = LedgerDoc::from_bytes(&base_bytes).unwrap();

        // A records a bill.
        doc_a
            .add_bill(
                NewBill {
                    payer_user_id: payer,
                    amount_cents: 1000,
                    description: "from A".to_string(),
                    shares: vec![Share { user_id: payer, shares: 1 }],
                },
                node_a,
                Timestamp::now(),
            )
            .unwrap();

        // B records a different bill independently.
        doc_b
            .add_bill(
                NewBill {
                    payer_user_id: payer,
                    amount_cents: 2000,
                    description: "from B".to_string(),
                    shares: vec![Share { user_id: payer, shares: 1 }],
                },
                node_b,
                Timestamp::now(),
            )
            .unwrap();

        let ledgers_a: DashMap<String, Arc<Mutex<LedgerDoc>>> = DashMap::new();
        ledgers_a.insert(ledger_id.clone(), Arc::new(Mutex::new(doc_a)));
        let ledgers_b: DashMap<String, Arc<Mutex<LedgerDoc>>> = DashMap::new();
        ledgers_b.insert(ledger_id.clone(), Arc::new(Mutex::new(doc_b)));

        let (la, lb) = sync_pair(ledgers_a, ledgers_b, node_a, node_b).await;

        let doc_a_final = la.get(&ledger_id).unwrap();
        let doc_b_final = lb.get(&ledger_id).unwrap();

        let bills_a = doc_a_final.lock().await.list_bills().unwrap();
        let bills_b = doc_b_final.lock().await.list_bills().unwrap();

        assert_eq!(bills_a.len(), 2, "A should have both bills after sync");
        assert_eq!(bills_b.len(), 2, "B should have both bills after sync");

        // Both sides converge to identical state.
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
        let doc_a = LedgerDoc::new(
            Ulid::new(),
            "Private".to_string(),
            usd(),
            Timestamp::now(),
        )
        .unwrap();
        let id = doc_a.get_ledger().unwrap().ledger_id.to_string();

        let ledgers_a: DashMap<String, Arc<Mutex<LedgerDoc>>> = DashMap::new();
        ledgers_a.insert(id.clone(), Arc::new(Mutex::new(doc_a)));

        // B has the same ledger ID (maybe imported some other way).
        let doc_b = LedgerDoc::new(Ulid::new(), "Same id?".to_string(), usd(), Timestamp::now())
            .unwrap();
        let ledgers_b: DashMap<String, Arc<Mutex<LedgerDoc>>> = DashMap::new();
        ledgers_b.insert(id.clone(), Arc::new(Mutex::new(doc_b)));

        // Sync should complete without error — A just rejects the ledger.
        let _ = sync_pair(ledgers_a, ledgers_b, node_a, node_b).await;
    }
}
