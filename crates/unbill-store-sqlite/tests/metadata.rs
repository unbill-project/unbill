use unbill_model::{Invitation, InviteToken, LedgerId, NodeId, Timestamp};
use unbill_storage::LedgerStore;
use unbill_store_fs::FsStore;
use unbill_store_memory::InMemoryStore;
use unbill_store_sqlite::SqliteStore;

fn invitation() -> Invitation {
    Invitation {
        token: InviteToken::generate(),
        ledger_id: LedgerId::from_u128(1),
        created_by_device: NodeId::new("host".into()),
        created_at: Timestamp::from_millis(1000),
        expires_at: Timestamp::from_millis(2000),
    }
}
async fn metadata_contract(store: &dyn LedgerStore) {
    assert!(store.list_device_labels().await.unwrap().is_empty());
    let a = NodeId::new("peer-a".into());
    let b = NodeId::new("peer-b".into());
    let (x, y) = tokio::join!(
        store.set_device_label(&a, Some("Laptop")),
        store.set_device_label(&b, Some("Phone"))
    );
    x.unwrap();
    y.unwrap();
    store.set_device_label(&a, Some("Desktop")).await.unwrap();
    assert_eq!(store.list_device_labels().await.unwrap().len(), 2);
    store.set_device_label(&a, None).await.unwrap();
    let labels = store.list_device_labels().await.unwrap();
    assert_eq!(labels.len(), 1);
    assert_eq!(labels["peer-b"], "Phone");
    assert!(store.list_pending_invitations().await.unwrap().is_empty());
    let first = invitation();
    let second = invitation();
    let (x, y) = tokio::join!(
        store.save_invitation(&first),
        store.save_invitation(&second)
    );
    x.unwrap();
    y.unwrap();
    assert_eq!(store.list_pending_invitations().await.unwrap().len(), 2);
    let (x, y) = tokio::join!(
        store.consume_invitation(first.token.as_str()),
        store.consume_invitation(first.token.as_str())
    );
    let consumed: Vec<_> = [x.unwrap(), y.unwrap()].into_iter().flatten().collect();
    assert_eq!(consumed.len(), 1);
    assert_eq!(consumed[0].token, first.token);
    assert_eq!(consumed[0].ledger_id, first.ledger_id);
    assert_eq!(consumed[0].created_by_device, first.created_by_device);
    assert_eq!(consumed[0].created_at, first.created_at);
    assert_eq!(consumed[0].expires_at, first.expires_at);
    assert_eq!(
        store.list_pending_invitations().await.unwrap()[0].token,
        second.token
    );
    assert!(!store.is_device_initialized().await.unwrap());
    let (x, y) = tokio::join!(store.create_secret_key(), store.create_secret_key());
    x.unwrap();
    y.unwrap();
    let id = store.get_device_id().await.unwrap();
    let key = store.get_secret_key().await.unwrap();
    store.create_secret_key().await.unwrap();
    assert_eq!(store.get_device_id().await.unwrap(), id);
    assert_eq!(
        store.get_secret_key().await.unwrap().as_bytes(),
        key.as_bytes()
    );
}

#[tokio::test]
async fn memory_metadata_contract() {
    metadata_contract(&InMemoryStore::default()).await;
}

#[tokio::test]
async fn filesystem_metadata_contract() {
    let dir = tempfile::tempdir().unwrap();
    let store = FsStore::open(dir.path().into()).unwrap();
    metadata_contract(&store).await;
    let id = store.get_device_id().await.unwrap();
    drop(store);
    let store = FsStore::open(dir.path().into()).unwrap();
    assert_eq!(store.get_device_id().await.unwrap(), id);
    assert_eq!(store.list_device_labels().await.unwrap()["peer-b"], "Phone");
    assert_eq!(store.list_pending_invitations().await.unwrap().len(), 1);
}

#[tokio::test]
async fn sqlite_metadata_contract() {
    let dir = tempfile::tempdir().unwrap();
    let store = SqliteStore::open(dir.path().into()).await.unwrap();
    metadata_contract(&store).await;
    let id = store.get_device_id().await.unwrap();
    drop(store);
    let store = SqliteStore::open(dir.path().into()).await.unwrap();
    assert_eq!(store.get_device_id().await.unwrap(), id);
    assert_eq!(store.list_device_labels().await.unwrap()["peer-b"], "Phone");
    assert_eq!(store.list_pending_invitations().await.unwrap().len(), 1);
}

#[tokio::test]
async fn existing_filesystem_metadata_is_read_without_conversion() {
    let dir = tempfile::tempdir().unwrap();
    let inv = invitation();
    std::fs::write(
        dir.path().join("device_labels.json"),
        br#"{"old-node":"Old laptop"}"#,
    )
    .unwrap();
    std::fs::write(
        dir.path().join("pending_invitations.json"),
        serde_json::to_vec(&std::collections::HashMap::from([(
            inv.token.to_string(),
            inv.clone(),
        )]))
        .unwrap(),
    )
    .unwrap();
    std::fs::write(dir.path().join("device_key.bin"), [7u8; 32]).unwrap();
    let store = FsStore::open(dir.path().into()).unwrap();
    assert_eq!(
        store.list_device_labels().await.unwrap()["old-node"],
        "Old laptop"
    );
    assert_eq!(
        store
            .consume_invitation(inv.token.as_str())
            .await
            .unwrap()
            .unwrap()
            .token,
        inv.token
    );
    assert_eq!(store.get_secret_key().await.unwrap().as_bytes(), &[7u8; 32]);
}

use diesel::{
    prelude::*,
    sql_types::{Binary, Text},
};
use diesel_migrations::{MigrationHarness, embed_migrations};

fn legacy_database(root: &std::path::Path) -> SqliteConnection {
    let mut conn =
        SqliteConnection::establish(root.join("unbill.sqlite3").to_str().unwrap()).unwrap();
    const MIGRATIONS: diesel_migrations::EmbeddedMigrations = embed_migrations!("migrations");
    conn.run_next_migration(MIGRATIONS).unwrap();
    conn
}
fn legacy_insert(conn: &mut SqliteConnection, key: &str, value: &[u8]) {
    diesel::sql_query("INSERT INTO device_metadata (key, value) VALUES (?, ?)")
        .bind::<Text, _>(key)
        .bind::<Binary, _>(value)
        .execute(conn)
        .unwrap();
}

#[tokio::test]
async fn migration_preserves_existing_metadata_and_removes_generic_table() {
    let dir = tempfile::tempdir().unwrap();
    let inv = invitation();
    let mut conn = legacy_database(dir.path());
    legacy_insert(&mut conn, "device_key.bin", &[9; 32]);
    legacy_insert(
        &mut conn,
        "device_labels.json",
        br#"{"old-node":"Old laptop"}"#,
    );
    legacy_insert(
        &mut conn,
        "pending_invitations.json",
        &serde_json::to_vec(&std::collections::HashMap::from([(
            inv.token.to_string(),
            inv.clone(),
        )]))
        .unwrap(),
    );
    drop(conn);
    let store = SqliteStore::open(dir.path().into()).await.unwrap();
    assert_eq!(store.get_secret_key().await.unwrap().as_bytes(), &[9; 32]);
    assert_eq!(
        store.list_device_labels().await.unwrap()["old-node"],
        "Old laptop"
    );
    let restored = store
        .consume_invitation(inv.token.as_str())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(restored.ledger_id, inv.ledger_id);
    assert_eq!(restored.created_by_device, inv.created_by_device);
    assert_eq!(restored.created_at, inv.created_at);
    assert_eq!(restored.expires_at, inv.expires_at);
    drop(store);
    let mut conn =
        SqliteConnection::establish(dir.path().join("unbill.sqlite3").to_str().unwrap()).unwrap();
    assert!(
        diesel::sql_query("SELECT * FROM device_metadata")
            .execute(&mut conn)
            .is_err()
    );
}

#[tokio::test]
async fn migration_rejects_unknown_or_malformed_metadata_without_losing_it() {
    for (key, value) in [
        ("unknown", b"data".as_slice()),
        ("device_key.bin", b"short".as_slice()),
        ("device_labels.json", br#"{"peer":123}"#.as_slice()),
        ("device_labels.json", b"[]".as_slice()),
        ("pending_invitations.json", br#"{"token":{}}"#.as_slice()),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let mut conn = legacy_database(dir.path());
        legacy_insert(&mut conn, key, value);
        drop(conn);
        assert!(
            SqliteStore::open(dir.path().into()).await.is_err(),
            "accepted {key}"
        );
        let mut conn =
            SqliteConnection::establish(dir.path().join("unbill.sqlite3").to_str().unwrap())
                .unwrap();
        #[derive(QueryableByName)]
        struct LegacyValue {
            #[diesel(sql_type = Binary)]
            value: Vec<u8>,
        }
        let original: LegacyValue =
            diesel::sql_query("SELECT value FROM device_metadata WHERE key = ?")
                .bind::<Text, _>(key)
                .get_result(&mut conn)
                .unwrap();
        assert_eq!(original.value, value);
    }
}
