use unbill_event::ServiceEvent;
use unbill_model::{Currency, LedgerDoc, LedgerId, LedgerMeta, NodeId, StorageError, Timestamp};
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
async fn independent_upserts_survive_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let store = SqliteStore::open(dir.path().into()).await.unwrap();
    let id = meta("First").ledger_id.to_string();
    assert!(store.list_ledgers().await.unwrap().is_empty());
    assert!(store.load_ledger(&id).await.unwrap().is_none());
    store.save_ledger(&id, &mut doc("First")).await.unwrap();
    assert!(store.list_ledgers().await.unwrap().is_empty());
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
    store
        .save_ledger(&id, &mut doc("Updated document"))
        .await
        .unwrap();
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
    assert_eq!(metas[0].name, "Updated metadata");
    assert_eq!(metas[0].currency.code(), "USD");
    assert_eq!(metas[0].updated_at.as_millis(), 2000);
    assert_eq!(
        store
            .load_ledger(&id)
            .await
            .unwrap()
            .unwrap()
            .get_ledger()
            .unwrap()
            .name,
        "Updated document"
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
async fn lock_rejects_second_owner_and_releases_on_drop() {
    let dir = tempfile::tempdir().unwrap();
    let store = SqliteStore::open(dir.path().into()).await.unwrap();
    assert!(matches!(SqliteStore::open(dir.path().into()).await,
        Err(StorageError::Io(e)) if e.kind() == std::io::ErrorKind::WouldBlock));
    drop(store);
    SqliteStore::open(dir.path().into())
        .await
        .expect("dropping the owner releases the lock");
}

#[tokio::test]
async fn invalid_database_fails_without_replacing_it() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("unbill.sqlite3");
    std::fs::write(&path, b"not a database").unwrap();
    assert!(SqliteStore::open(dir.path().into()).await.is_err());
    assert_eq!(std::fs::read(path).unwrap(), b"not a database");
}
