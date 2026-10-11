use diesel::{
    connection::SimpleConnection,
    prelude::*,
    sql_types::{BigInt, Binary},
};
use diesel_migrations::{MigrationHarness, embed_migrations};
use unbill_model::{Currency, LedgerDoc, LedgerId, Timestamp};
use unbill_storage::LedgerStore;
use unbill_store_sqlite::SqliteStore;

const MIGRATIONS: diesel_migrations::EmbeddedMigrations = embed_migrations!("migrations");

fn connection(root: &std::path::Path) -> SqliteConnection {
    SqliteConnection::establish(root.join("unbill.sqlite3").to_str().unwrap()).unwrap()
}

#[derive(QueryableByName, Debug, PartialEq)]
struct Bytes {
    #[diesel(sql_type = Binary)]
    metadata: Vec<u8>,
    #[diesel(sql_type = Binary)]
    document: Vec<u8>,
}
fn bytes(conn: &mut SqliteConnection) -> Bytes {
    diesel::sql_query("SELECT metadata, document FROM ledgers")
        .get_result(conn)
        .unwrap()
}
#[derive(QueryableByName)]
struct Count {
    #[diesel(sql_type = BigInt)]
    count: i64,
}
fn count(conn: &mut SqliteConnection, sql: &str) -> i64 {
    diesel::sql_query(sql)
        .get_result::<Count>(conn)
        .unwrap()
        .count
}

/// Downgrade only the new, empty archive table to reproduce the released schema
/// with real persisted documents, identity and invitation data.
async fn old_database(
    root: &std::path::Path,
) -> (LedgerId, Bytes, Vec<u8>, unbill_model::InviteToken) {
    let store = SqliteStore::open(root.to_owned()).await.unwrap();
    let id = LedgerId::from_u128(1);
    let mut doc = LedgerDoc::new(
        id,
        "Existing".into(),
        Currency::from_code("USD").unwrap(),
        Timestamp::from_millis(1000),
    )
    .unwrap();
    store.save_ledger(&id.to_string(), &mut doc).await.unwrap();
    store.create_secret_key().await.unwrap();
    let key = store.get_secret_key().await.unwrap().as_bytes().to_vec();
    let invitation = store
        .create_invitation(
            id,
            &store.get_device_id().await.unwrap(),
            Timestamp::from_millis(1000),
            Timestamp::from_millis(2000),
        )
        .await
        .unwrap();
    let token = invitation.token.clone();
    drop(store);
    let mut conn = connection(root);
    conn.revert_last_migration(MIGRATIONS).unwrap();
    (id, bytes(&mut conn), key, token)
}

#[tokio::test]
async fn upgrade_preserves_bytes_identity_invitations_and_defaults_active() {
    let root = tempfile::tempdir().unwrap();
    let (id, before, key, token) = old_database(root.path()).await;
    let store = SqliteStore::open(root.path().to_owned()).await.unwrap();
    assert_eq!(bytes(&mut connection(root.path())), before);
    assert_eq!(
        store.get_secret_key().await.unwrap().as_bytes().to_vec(),
        key
    );
    assert_eq!(
        store
            .list_pending_invitations()
            .await
            .unwrap()
            .first()
            .unwrap()
            .token,
        token
    );
    assert!(
        !store
            .list_ledgers()
            .await
            .unwrap()
            .first()
            .unwrap()
            .archived
    );
    let before_hash = store
        .load_ledger(&id.to_string())
        .await
        .unwrap()
        .unwrap()
        .state_hash();
    let stale_meta = store.list_ledgers().await.unwrap().remove(0);
    let mut events = store.subscribe();
    let revision_before = count(
        &mut connection(root.path()),
        "SELECT clock AS count FROM storage_revisions",
    );
    store.set_ledger_archived(id, true).await.unwrap();
    assert!(
        matches!(events.recv().await.unwrap(), unbill_event::ServiceEvent::LedgerUpdated { ledger_id } if ledger_id == id.to_string())
    );
    assert!(
        count(
            &mut connection(root.path()),
            "SELECT clock AS count FROM storage_revisions"
        ) > revision_before
    );
    assert_eq!(bytes(&mut connection(root.path())), before);
    store.save_ledger_meta(&stale_meta).await.unwrap();
    let mut doc = store.load_ledger(&id.to_string()).await.unwrap().unwrap();
    store.save_ledger(&id.to_string(), &mut doc).await.unwrap();
    assert!(
        store
            .list_ledgers()
            .await
            .unwrap()
            .first()
            .unwrap()
            .archived
    );
    assert_eq!(doc.state_hash(), before_hash);
    drop(store);
    let reopened = SqliteStore::open(root.path().to_owned()).await.unwrap();
    assert!(
        reopened
            .list_ledgers()
            .await
            .unwrap()
            .first()
            .unwrap()
            .archived
    );
    reopened.set_ledger_archived(id, false).await.unwrap();
    assert!(
        !reopened
            .list_ledgers()
            .await
            .unwrap()
            .first()
            .unwrap()
            .archived
    );
    assert_eq!(
        reopened
            .load_ledger(&id.to_string())
            .await
            .unwrap()
            .unwrap()
            .state_hash(),
        before_hash
    );
    assert!(
        reopened
            .set_ledger_archived(LedgerId::from_u128(2), true)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn failed_migration_rolls_back_schema_and_preserves_existing_data() {
    let root = tempfile::tempdir().unwrap();
    let (_, before, key, token) = old_database(root.path()).await;
    let mut conn = connection(root.path());
    // The second CREATE TRIGGER fails after the table and first trigger were
    // created. Neither partial schema nor a successful migration record may remain.
    conn.batch_execute("CREATE TRIGGER ledger_archive_update_revision AFTER UPDATE ON ledgers BEGIN SELECT 1; END;").unwrap();
    assert!(SqliteStore::open(root.path().to_owned()).await.is_err());
    assert_eq!(bytes(&mut conn), before);
    assert_eq!(
        count(
            &mut conn,
            "SELECT count(*) AS count FROM sqlite_master WHERE name IN ('ledger_archives', 'ledger_archive_insert_revision')"
        ),
        0
    );
    assert_eq!(
        count(
            &mut conn,
            "SELECT count(*) AS count FROM __diesel_schema_migrations WHERE version = '00000000000003'"
        ),
        0
    );
    conn.batch_execute("DROP TRIGGER ledger_archive_update_revision;")
        .unwrap();
    drop(conn);
    let store = SqliteStore::open(root.path().to_owned()).await.unwrap();
    assert_eq!(
        store.get_secret_key().await.unwrap().as_bytes().to_vec(),
        key
    );
    assert_eq!(
        store
            .list_pending_invitations()
            .await
            .unwrap()
            .first()
            .unwrap()
            .token,
        token
    );
}

#[tokio::test]
async fn archive_is_device_local_and_memory_store_obeys_the_same_contract() {
    let left = unbill_store_memory::InMemoryStore::default();
    let right = unbill_store_memory::InMemoryStore::default();
    let id = LedgerId::from_u128(1);
    let mut doc = LedgerDoc::new(
        id,
        "Shared".into(),
        Currency::from_code("USD").unwrap(),
        Timestamp::from_millis(1000),
    )
    .unwrap();
    left.save_ledger(&id.to_string(), &mut doc).await.unwrap();
    right.save_ledger(&id.to_string(), &mut doc).await.unwrap();
    left.set_ledger_archived(id, true).await.unwrap();
    left.save_ledger(&id.to_string(), &mut doc).await.unwrap();
    assert!(left.list_ledgers().await.unwrap().first().unwrap().archived);
    assert!(
        !right
            .list_ledgers()
            .await
            .unwrap()
            .first()
            .unwrap()
            .archived
    );
    left.set_ledger_archived(id, false).await.unwrap();
    assert!(!left.list_ledgers().await.unwrap().first().unwrap().archived);
}

#[tokio::test]
async fn externally_committed_archive_changes_notify_observers() {
    let root = tempfile::tempdir().unwrap();
    let store = SqliteStore::open(root.path().to_owned()).await.unwrap();
    let id = LedgerId::from_u128(1);
    let mut doc = LedgerDoc::new(
        id,
        "Observed".into(),
        Currency::from_code("USD").unwrap(),
        Timestamp::from_millis(1000),
    )
    .unwrap();
    store.save_ledger(&id.to_string(), &mut doc).await.unwrap();
    let mut events = store.subscribe();
    let mut conn = connection(root.path());
    conn.batch_execute("INSERT INTO ledger_archives (ledger_id, archived) VALUES ('00000000000000000000000001', 1);").unwrap();
    // The observer can coalesce document and archive revisions; wait until its
    // notification corresponds to the already committed archive preference.
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        loop {
            if matches!(events.recv().await.unwrap(), unbill_event::ServiceEvent::LedgerUpdated { ledger_id } if ledger_id == id.to_string())
                && store.list_ledgers().await.unwrap().first().unwrap().archived {
                break;
            }
        }
    }).await.unwrap();
}
