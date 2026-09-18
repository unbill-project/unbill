use unbill_event::ServiceEvent;
use unbill_model::{Currency, LedgerDoc, LedgerId, LedgerMeta, StorageError, Timestamp};
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
    store.save_device_meta("labels", b"old").await.unwrap();
    store.save_device_meta("labels", b"new").await.unwrap();
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
        store.load_device_meta("labels").await.unwrap(),
        Some(b"new".to_vec())
    );
    assert!(store.load_device_meta("missing").await.unwrap().is_none());
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
    store
        .save_device_meta("device_key.bin", b"invalid")
        .await
        .unwrap();
    assert!(matches!(
        store.get_secret_key().await,
        Err(StorageError::Serialization(_))
    ));
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
    assert!(SqliteStore::open(dir.path().into()).await.is_ok());
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
async fn lock_child_process() {
    let Some(root) = std::env::var_os("UNBILL_SQLITE_LOCK_TEST_ROOT") else {
        return;
    };
    assert!(matches!(SqliteStore::open(root.into()).await,
        Err(StorageError::Io(e)) if e.kind() == std::io::ErrorKind::WouldBlock));
}

#[tokio::test]
async fn lock_is_enforced_across_processes() {
    let dir = tempfile::tempdir().unwrap();
    let _store = SqliteStore::open(dir.path().into()).await.unwrap();
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "lock_child_process"])
        .env("UNBILL_SQLITE_LOCK_TEST_ROOT", dir.path())
        .status()
        .unwrap();
    assert!(status.success());
}
