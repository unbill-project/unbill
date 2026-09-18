use unbill_event::ServiceEvent;
use unbill_model::{Currency, LedgerDoc, LedgerId, LedgerMeta, NodeId, Timestamp};
use unbill_storage::LedgerStore;
use unbill_store_sqlite::SqliteStore;

fn meta(name: &str) -> LedgerMeta {
    LedgerMeta {
        ledger_id: LedgerId::from_u128(1),
        name: name.into(),
        currency: Currency::from_code("USD").unwrap(),
        created_at: Timestamp::from_millis(1000),
        updated_at: Timestamp::from_millis(2000),
    }
}
fn doc(name: &str) -> LedgerDoc {
    LedgerDoc::new(
        LedgerId::from_u128(1),
        name.into(),
        Currency::from_code("USD").unwrap(),
        Timestamp::from_millis(1000),
    )
    .unwrap()
}

#[tokio::test]
async fn document_metadata_is_authoritative_and_survives_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let store = SqliteStore::open(dir.path().into()).await.unwrap();
    let id = meta("First").ledger_id.to_string();
    assert!(store.list_ledgers().await.unwrap().is_empty());
    assert!(store.load_ledger(&id).await.unwrap().is_none());
    store.save_ledger(&id, &mut doc("First")).await.unwrap();
    assert_eq!(store.list_ledgers().await.unwrap()[0].name, "First");
    store.save_ledger_meta(&meta("First")).await.unwrap();
    store
        .save_ledger_meta(&meta("Updated metadata"))
        .await
        .unwrap();
    assert_eq!(
        store
            .load_ledger(&id)
            .await
            .unwrap()
            .unwrap()
            .get_ledger()
            .unwrap()
            .name,
        "First"
    );
    store.save_ledger(&id, &mut doc("First")).await.unwrap();
    let mut second = meta("Metadata only");
    second.ledger_id = LedgerId::from_u128(2);
    store.save_ledger_meta(&second).await.unwrap();
    assert!(
        store
            .load_ledger(&second.ledger_id.to_string())
            .await
            .unwrap()
            .is_none()
    );
    let node = NodeId::new("test-node".into());
    store.set_device_label(&node, Some("old")).await.unwrap();
    store.set_device_label(&node, Some("new")).await.unwrap();
    drop(store);
    let store = SqliteStore::open(dir.path().into()).await.unwrap();
    let metas = store.list_ledgers().await.unwrap();
    assert_eq!(metas.len(), 2);
    assert_eq!(metas[0].name, "First");
    assert_eq!(metas[0].currency.code(), "USD");
    assert!(metas[0].updated_at.as_millis() >= 2000);
    assert_eq!(
        store
            .load_ledger(&id)
            .await
            .unwrap()
            .unwrap()
            .get_ledger()
            .unwrap()
            .name,
        "First"
    );
    assert_eq!(
        store
            .list_device_labels()
            .await
            .unwrap()
            .get("test-node")
            .map(String::as_str),
        Some("new")
    );
}

#[tokio::test]
async fn identity_is_idempotent_and_persistent() {
    let dir = tempfile::tempdir().unwrap();
    let store = SqliteStore::open(dir.path().into()).await.unwrap();
    assert!(!store.is_device_initialized().await.unwrap());
    assert!(store.get_secret_key().await.is_err());
    let (a, b) = tokio::join!(store.create_secret_key(), store.create_secret_key());
    a.unwrap();
    b.unwrap();
    let secret = store.get_secret_key().await.unwrap();
    let id = store.get_device_id().await.unwrap();
    store.create_secret_key().await.unwrap();
    assert_eq!(
        store.get_secret_key().await.unwrap().as_bytes(),
        secret.as_bytes()
    );
    drop(store);
    let store = SqliteStore::open(dir.path().into()).await.unwrap();
    assert!(store.is_device_initialized().await.unwrap());
    assert_eq!(store.get_device_id().await.unwrap(), id);
}

#[tokio::test]
async fn save_emits_event_after_persistence() {
    let dir = tempfile::tempdir().unwrap();
    let store = SqliteStore::open(dir.path().into()).await.unwrap();
    let mut events = store.subscribe();
    let id = meta("Test").ledger_id.to_string();
    store.save_ledger(&id, &mut doc("Test")).await.unwrap();
    assert!(
        matches!(events.try_recv().unwrap(), ServiceEvent::LedgerUpdated { ledger_id } if ledger_id == id)
    );
    assert!(store.load_ledger(&id).await.unwrap().is_some());
}

#[tokio::test]
async fn sqlite_does_not_create_or_acquire_the_directory_lock() {
    let dir = tempfile::tempdir().unwrap();
    let first = SqliteStore::open(dir.path().into()).await.unwrap();
    assert!(!dir.path().join("unbill.lock").exists());
    let fs = unbill_store_fs::FsStore::open(dir.path().into()).unwrap();
    let second = SqliteStore::open(dir.path().into()).await.unwrap();
    first
        .save_ledger(&meta("Test").ledger_id.to_string(), &mut doc("Test"))
        .await
        .unwrap();
    assert_eq!(second.list_ledgers().await.unwrap()[0].name, "Test");
    drop(second);
    drop(first);
    // SQLite did not release the filesystem backend's exclusive lock.
    assert!(unbill_store_fs::FsStore::open(dir.path().into()).is_err());
    drop(fs);
}

#[tokio::test]
async fn invalid_database_fails_without_replacing_it() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("unbill.sqlite3");
    std::fs::write(&path, b"not a database").unwrap();
    assert!(SqliteStore::open(dir.path().into()).await.is_err());
    assert_eq!(std::fs::read(path).unwrap(), b"not a database");
}

#[tokio::test]
async fn incompatible_documents_are_rejected_without_losing_caller_state() {
    let dir = tempfile::tempdir().unwrap();
    let store = SqliteStore::open(dir.path().into()).await.unwrap();
    let id = meta("Original").ledger_id.to_string();
    store.save_ledger(&id, &mut doc("Original")).await.unwrap();
    let mut incompatible = doc("Different");
    assert!(store.save_ledger(&id, &mut incompatible).await.is_err());
    assert_eq!(incompatible.get_ledger().unwrap().name, "Different");
    let wrong_id = LedgerId::from_u128(99).to_string();
    assert!(
        store
            .save_ledger(&wrong_id, &mut incompatible)
            .await
            .is_err()
    );
    assert!(store.load_ledger(&wrong_id).await.unwrap().is_none());
    assert_eq!(store.list_ledgers().await.unwrap()[0].name, "Original");
    assert_eq!(
        store
            .load_ledger(&id)
            .await
            .unwrap()
            .unwrap()
            .get_ledger()
            .unwrap()
            .name,
        "Original"
    );
}

#[tokio::test]
async fn actor_forwarding_survives_no_listeners_and_recovers_lag() {
    use std::{sync::Arc, time::Duration};
    use unbill_storage::StoreServer;
    use unbill_store_memory::InMemoryStore;
    let raw = Arc::new(InMemoryStore::default());
    let actor = StoreServer::spawn(raw.clone());
    let id = meta("Test").ledger_id.to_string();
    raw.save_ledger_meta(&meta("Test")).await.unwrap();
    raw.save_ledger(&id, &mut doc("Test")).await.unwrap();
    tokio::task::yield_now().await;
    let mut events = actor.subscribe();
    raw.set_device_label(&NodeId::new("peer".into()), Some("label"))
        .await
        .unwrap();
    assert!(matches!(
        tokio::time::timeout(Duration::from_secs(2), events.recv())
            .await
            .unwrap()
            .unwrap(),
        ServiceEvent::DeviceLabelsUpdated
    ));
    // These in-memory operations do not yield, so overflow the raw-store receiver.
    for n in 0..300 {
        raw.set_device_label(&NodeId::new("peer".into()), Some(&n.to_string()))
            .await
            .unwrap();
    }
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            match events.recv().await {
                Ok(ServiceEvent::LedgerUpdated { ledger_id }) if ledger_id == id => break,
                Err(tokio::sync::broadcast::error::RecvError::Closed) => panic!("forwarder closed"),
                _ => {}
            }
        }
    })
    .await
    .unwrap();
    raw.set_device_label(&NodeId::new("peer".into()), None)
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if matches!(events.recv().await, Ok(ServiceEvent::DeviceLabelsUpdated)) {
                break;
            }
        }
    })
    .await
    .unwrap();
}
