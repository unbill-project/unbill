// Keep process spawning separate from tests that immediately reopen a dropped store:
// a fork can briefly inherit unrelated lock descriptors before exec closes them.
use unbill_model::StorageError;
use unbill_store_sqlite::SqliteStore;

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
