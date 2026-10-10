//! Cross-process directory ownership, including release after a crashed owner.
#![cfg(not(any(
    target_os = "android",
    all(target_os = "ios", not(target_abi = "macabi"))
)))]
use std::{
    io::{BufRead, BufReader, Write},
    path::Path,
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
};
use unbill_model::StorageError;
use unbill_storage::LedgerStore;
use unbill_store_sqlite::SqliteStore;

fn signal(message: &str) {
    println!("LOCK:{message}");
    std::io::stdout().flush().unwrap();
}
fn proceed() {
    let mut line = String::new();
    assert!(std::io::stdin().read_line(&mut line).unwrap() > 0);
}

#[tokio::test]
async fn worker() {
    let Ok(root) = std::env::var("UNBILL_LOCK_TEST_ROOT") else {
        return;
    };
    signal("boot");
    proceed();
    if std::env::var("UNBILL_LOCK_TEST_ACTION").as_deref() == Ok("sql_writer") {
        use diesel::{Connection, connection::SimpleConnection};
        let mut connection = diesel::SqliteConnection::establish(
            Path::new(&root).join("unbill.sqlite3").to_str().unwrap(),
        )
        .unwrap();
        connection.batch_execute("BEGIN IMMEDIATE; INSERT INTO device_identity (id, secret_key) VALUES (1, zeroblob(32));").unwrap();
        signal("sql_locked");
        proceed();
        connection.batch_execute("ROLLBACK;").unwrap();
        return;
    }
    match SqliteStore::open(root.into()).await {
        Ok(store) => {
            store.create_secret_key().await.unwrap();
            signal(&format!("owner:{}", store.get_device_id().await.unwrap()));
            proceed();
            drop(store);
            signal("released");
        }
        Err(StorageError::Io(error)) if error.kind() == std::io::ErrorKind::WouldBlock => {
            signal("busy")
        }
        Err(error) => panic!("unexpected startup error: {error}"),
    }
}

struct Process {
    child: Child,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
}
impl Process {
    fn spawn(root: &Path) -> Self {
        Self::spawn_action(root, "owner")
    }
    fn spawn_action(root: &Path, action: &str) -> Self {
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "worker", "--nocapture"])
            .env("UNBILL_LOCK_TEST_ROOT", root)
            .env("UNBILL_LOCK_TEST_ACTION", action)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();
        let input = child.stdin.take().unwrap();
        let output = BufReader::new(child.stdout.take().unwrap());
        let mut process = Self {
            child,
            input,
            output,
        };
        assert_eq!(process.read(), "boot");
        process
    }
    fn go(&mut self) {
        writeln!(self.input, "go").unwrap();
        self.input.flush().unwrap();
    }
    fn read(&mut self) -> String {
        loop {
            let mut line = String::new();
            assert!(
                self.output.read_line(&mut line).unwrap() > 0,
                "worker exited before signalling"
            );
            if let Some(value) = line.trim().strip_prefix("LOCK:") {
                return value.to_owned();
            }
        }
    }
    fn finish(&mut self) {
        assert!(self.child.wait().unwrap().success());
    }
}
impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn simultaneous_startup_has_exactly_one_owner() {
    let dir = tempfile::tempdir().unwrap();
    let mut a = Process::spawn(dir.path());
    let mut b = Process::spawn(dir.path());
    a.go();
    b.go();
    let first = a.read();
    let second = b.read();
    let (owner, loser, identity) = if let Some(identity) = first.strip_prefix("owner:") {
        assert_eq!(second, "busy");
        (&mut a, &mut b, identity.to_owned())
    } else {
        assert_eq!(first, "busy");
        (
            &mut b,
            &mut a,
            second.strip_prefix("owner:").unwrap().to_owned(),
        )
    };
    loser.finish();
    owner.go();
    assert_eq!(owner.read(), "released");
    owner.finish();
    let mut next = Process::spawn(dir.path());
    next.go();
    assert_eq!(next.read(), format!("owner:{identity}"));
    next.go();
    assert_eq!(next.read(), "released");
    next.finish();
}

#[test]
fn crashed_owner_releases_lock_without_deleting_lock_file() {
    let dir = tempfile::tempdir().unwrap();
    let mut owner = Process::spawn(dir.path());
    owner.go();
    let identity = owner.read();
    assert!(identity.starts_with("owner:"));
    let mut contender = Process::spawn(dir.path());
    contender.go();
    assert_eq!(contender.read(), "busy");
    contender.finish();
    owner.child.kill().unwrap();
    owner.child.wait().unwrap();
    assert!(dir.path().join("unbill.lock").exists());
    let mut next = Process::spawn(dir.path());
    next.go();
    assert_eq!(next.read(), identity);
    next.go();
    assert_eq!(next.read(), "released");
    next.finish();
}

#[tokio::test]
async fn sqlite_writer_timeout_preserves_state_and_crashed_transaction_rolls_back() {
    use unbill_model::{Currency, LedgerDoc, LedgerId, NewUser, Timestamp, UserId};
    let dir = tempfile::tempdir().unwrap();
    let store = SqliteStore::open(dir.path().into()).await.unwrap();
    let id = LedgerId::from_u128(1);
    let mut document = LedgerDoc::new(
        id,
        "Shared".into(),
        Currency::from_code("USD").unwrap(),
        Timestamp::from_millis(1000),
    )
    .unwrap();
    store
        .save_ledger(&id.to_string(), &mut document)
        .await
        .unwrap();
    let mut writer = Process::spawn_action(dir.path(), "sql_writer");
    writer.go();
    assert_eq!(writer.read(), "sql_locked");
    document
        .add_user(
            NewUser {
                user_id: UserId::from_u128(1),
                display_name: "Alice".into(),
            },
            Timestamp::from_millis(2000),
        )
        .unwrap();
    let started = std::time::Instant::now();
    assert!(
        store
            .save_ledger(&id.to_string(), &mut document)
            .await
            .is_err()
    );
    assert!(started.elapsed() >= std::time::Duration::from_secs(4));
    assert_eq!(document.list_users().unwrap().len(), 1);
    assert!(
        store
            .load_ledger(&id.to_string())
            .await
            .unwrap()
            .unwrap()
            .list_users()
            .unwrap()
            .is_empty()
    );
    writer.child.kill().unwrap();
    writer.child.wait().unwrap();
    assert!(!store.is_device_initialized().await.unwrap());
    store
        .save_ledger(&id.to_string(), &mut document)
        .await
        .unwrap();
    drop(store);
    let reopened = SqliteStore::open(dir.path().into()).await.unwrap();
    assert_eq!(
        reopened
            .load_ledger(&id.to_string())
            .await
            .unwrap()
            .unwrap()
            .list_users()
            .unwrap()
            .len(),
        1
    );
}
