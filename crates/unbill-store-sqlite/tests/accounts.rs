use argon2::{Argon2, PasswordHash, PasswordVerifier};
use diesel::{Connection as _, connection::SimpleConnection};
use unbill_model::{Currency, LedgerDoc, LedgerId, Timestamp};
use unbill_storage::LedgerStore as _;
use unbill_store_sqlite::SqliteStore;

#[tokio::test]
async fn fresh_database_never_seeds_admin() {
    let root = tempfile::tempdir().unwrap();
    let store = SqliteStore::open(root.path().to_path_buf()).await.unwrap();
    assert!(store.list_accounts().await.unwrap().is_empty());
    drop(store);
    let reopened = SqliteStore::open(root.path().to_path_buf()).await.unwrap();
    assert!(reopened.list_accounts().await.unwrap().is_empty());
}

#[tokio::test]
async fn legacy_upgrade_seeds_admin_with_all_existing_ledgers() {
    let root = tempfile::tempdir().unwrap();
    let store = SqliteStore::open(root.path().to_path_buf()).await.unwrap();
    let id = LedgerId::new();
    let mut doc = LedgerDoc::new(
        id,
        "Existing".into(),
        Currency::from_code("USD").unwrap(),
        Timestamp::now().unwrap(),
    )
    .unwrap();
    store.save_ledger(&id.to_string(), &mut doc).await.unwrap();
    drop(store);

    let path = root.path().join("unbill.sqlite3");
    let mut conn = diesel::SqliteConnection::establish(path.to_str().unwrap()).unwrap();
    conn.batch_execute("DROP TABLE account_sessions; DROP TABLE account_ledgers; DROP TABLE accounts; DELETE FROM __diesel_schema_migrations WHERE version = '00000000000004';").unwrap();
    drop(conn);

    let upgraded = SqliteStore::open(root.path().to_path_buf()).await.unwrap();
    assert_eq!(upgraded.list_accounts().await.unwrap(), vec!["admin"]);
    assert!(upgraded.account_has_ledger("admin", id).await.unwrap());
    let account = upgraded.get_account("admin").await.unwrap().unwrap();
    let hash = PasswordHash::new(&account.password_hash).unwrap();
    Argon2::default().verify_password(b"admin", &hash).unwrap();
}
