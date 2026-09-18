//! Real-process concurrency tests. Stdin/stdout barriers deliberately overlap reads and writes.
use diesel::{connection::SimpleConnection, prelude::*};
use std::{
    io::{BufRead, BufReader, Write},
    path::Path,
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
    sync::Arc,
    time::Duration,
};
use unbill_event::ServiceEvent;
use unbill_model::{Currency, LedgerDoc, LedgerId, LedgerMeta, NewUser, NodeId, Timestamp, UserId};
use unbill_storage::{LedgerStore, StoreServer};
use unbill_store_sqlite::SqliteStore;

fn id() -> String {
    LedgerId::from_u128(1).to_string()
}
fn document() -> LedgerDoc {
    LedgerDoc::new(
        LedgerId::from_u128(1),
        "Shared".into(),
        Currency::from_code("USD").unwrap(),
        Timestamp::from_millis(1000),
    )
    .unwrap()
}
fn signal(message: &str) {
    println!("MP:{message}");
    std::io::stdout().flush().unwrap();
}
fn proceed() {
    let mut line = String::new();
    assert!(std::io::stdin().read_line(&mut line).unwrap() > 0);
}

#[tokio::test]
async fn worker() {
    let Ok(root) = std::env::var("UNBILL_MP_ROOT") else {
        return;
    };
    let action = std::env::var("UNBILL_MP_ACTION").unwrap();
    let role = std::env::var("UNBILL_MP_ROLE").unwrap();
    signal("boot");
    proceed();
    if action == "lock" {
        let mut conn =
            SqliteConnection::establish(Path::new(&root).join("unbill.sqlite3").to_str().unwrap())
                .unwrap();
        conn.batch_execute("BEGIN IMMEDIATE; INSERT INTO device_labels (node_id, label) VALUES ('uncommitted', 'bad');").unwrap();
        signal("locked");
        proceed();
        conn.batch_execute("ROLLBACK;").unwrap();
        signal("done");
        return;
    }
    let store = SqliteStore::open(root.into()).await.unwrap();
    if action == "init" {
        store.create_secret_key().await.unwrap();
        signal(&format!("id:{}", store.get_device_id().await.unwrap()));
        return;
    }
    let mut doc = if action == "merge" {
        Some(store.load_ledger(&id()).await.unwrap().unwrap())
    } else {
        None
    };
    signal("ready");
    proceed();
    match action.as_str() {
        "merge" => {
            let doc = doc.as_mut().unwrap();
            doc.add_user(
                NewUser {
                    user_id: UserId::from_u128(role.parse().unwrap()),
                    display_name: role.clone(),
                },
                Timestamp::from_millis(2000),
            )
            .unwrap();
            store.save_ledger(&id(), doc).await.unwrap();
            assert!(
                doc.list_users()
                    .unwrap()
                    .iter()
                    .any(|user| user.display_name == role)
            );
            store
                .save_ledger_meta(&LedgerMeta {
                    ledger_id: LedgerId::from_u128(1),
                    name: "Stale".into(),
                    currency: Currency::from_code("EUR").unwrap(),
                    created_at: Timestamp::from_millis(0),
                    updated_at: Timestamp::from_millis(1),
                })
                .await
                .unwrap();
            store
                .set_device_label(&NodeId::new(role.clone()), Some(&role))
                .await
                .unwrap();
            store
                .set_device_label(&NodeId::new("shared-label".into()), Some(&role))
                .await
                .unwrap();
            signal("done");
        }
        "consume" => {
            let invitation = store.consume_invitation(&role).await.unwrap();
            signal(if invitation.is_some() { "some" } else { "none" });
        }
        "notify" => {
            store.create_secret_key().await.unwrap();
            store
                .set_device_label(&NodeId::new("peer".into()), Some("label"))
                .await
                .unwrap();
            let inv = store
                .create_invitation(
                    LedgerId::from_u128(1),
                    &NodeId::new("host".into()),
                    Timestamp::from_millis(1),
                    Timestamp::from_millis(2),
                )
                .await
                .unwrap();
            store.consume_invitation(inv.token.as_str()).await.unwrap();
            store.save_ledger(&id(), &mut document()).await.unwrap();
            signal("done");
        }
        _ => panic!("unknown worker action"),
    }
}

struct Process {
    child: Child,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
}
impl Process {
    fn spawn(root: &Path, action: &str, role: &str) -> Self {
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "worker", "--nocapture"])
            .env("UNBILL_MP_ROOT", root)
            .env("UNBILL_MP_ACTION", action)
            .env("UNBILL_MP_ROLE", role)
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
            if let Some(value) = line.trim().strip_prefix("MP:") {
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

#[tokio::test]
async fn simultaneous_first_open_and_identity_initialization() {
    let dir = tempfile::tempdir().unwrap();
    let mut a = Process::spawn(dir.path(), "init", "a");
    let mut b = Process::spawn(dir.path(), "init", "b");
    a.go();
    b.go();
    let first = a.read();
    let second = b.read();
    assert!(first.starts_with("id:"));
    assert_eq!(first, second);
    a.finish();
    b.finish();
    let store = SqliteStore::open(dir.path().into()).await.unwrap();
    assert_eq!(
        format!("id:{}", store.get_device_id().await.unwrap()),
        first
    );
}

#[tokio::test]
async fn stale_documents_merge_and_metadata_never_reverts() {
    let dir = tempfile::tempdir().unwrap();
    let store = SqliteStore::open(dir.path().into()).await.unwrap();
    store.save_ledger(&id(), &mut document()).await.unwrap();
    let before = store.list_ledgers().await.unwrap()[0].updated_at;
    let mut a = Process::spawn(dir.path(), "merge", "1");
    let mut b = Process::spawn(dir.path(), "merge", "2");
    a.go();
    b.go();
    assert_eq!(a.read(), "ready");
    assert_eq!(b.read(), "ready");
    a.go();
    assert_eq!(a.read(), "done");
    a.finish();
    b.go();
    assert_eq!(b.read(), "done");
    b.finish();
    let users = store
        .load_ledger(&id())
        .await
        .unwrap()
        .unwrap()
        .list_users()
        .unwrap();
    assert_eq!(users.len(), 2);
    let meta = &store.list_ledgers().await.unwrap()[0];
    assert_eq!(meta.name, "Shared");
    assert_eq!(meta.currency.code(), "USD");
    assert_eq!(meta.created_at.as_millis(), 1000);
    assert!(meta.updated_at >= before);
    let labels = store.list_device_labels().await.unwrap();
    assert_eq!(labels["1"], "1");
    assert_eq!(labels["2"], "2");
    assert_eq!(labels["shared-label"], "2");
}

#[tokio::test]
async fn only_one_process_consumes_an_invitation() {
    let dir = tempfile::tempdir().unwrap();
    let store = SqliteStore::open(dir.path().into()).await.unwrap();
    let inv = store
        .create_invitation(
            LedgerId::from_u128(1),
            &NodeId::new("host".into()),
            Timestamp::from_millis(1),
            Timestamp::from_millis(2),
        )
        .await
        .unwrap();
    let mut a = Process::spawn(dir.path(), "consume", inv.token.as_str());
    let mut b = Process::spawn(dir.path(), "consume", inv.token.as_str());
    a.go();
    b.go();
    assert_eq!(a.read(), "ready");
    assert_eq!(b.read(), "ready");
    a.go();
    b.go();
    let mut results = [a.read(), b.read()];
    results.sort();
    assert_eq!(results, ["none", "some"]);
    a.finish();
    b.finish();
    assert!(store.list_pending_invitations().await.unwrap().is_empty());
}

#[tokio::test]
async fn other_process_commits_notify_every_record_type() {
    let dir = tempfile::tempdir().unwrap();
    let store = SqliteStore::open(dir.path().into()).await.unwrap();
    let mut events = store.subscribe();
    let mut process = Process::spawn(dir.path(), "notify", "");
    process.go();
    assert_eq!(process.read(), "ready");
    process.go();
    assert_eq!(process.read(), "done");
    process.finish();
    let mut seen = [false; 4];
    tokio::time::timeout(Duration::from_secs(5), async {
        while seen.contains(&false) {
            match events.recv().await.unwrap() {
                ServiceEvent::LedgerUpdated { ledger_id } => {
                    assert!(store.load_ledger(&ledger_id).await.unwrap().is_some());
                    seen[0] = true;
                }
                ServiceEvent::DeviceIdentityInitialized => seen[1] = true,
                ServiceEvent::DeviceLabelsUpdated => seen[2] = true,
                ServiceEvent::PendingInvitationsUpdated => seen[3] = true,
                _ => {}
            }
        }
    })
    .await
    .expect("missing cross-process event");
}

#[tokio::test]
async fn timeout_preserves_caller_and_crashed_writer_rolls_back() {
    let dir = tempfile::tempdir().unwrap();
    let store = Arc::new(SqliteStore::open(dir.path().into()).await.unwrap());
    store.save_ledger(&id(), &mut document()).await.unwrap();
    let mut process = Process::spawn(dir.path(), "lock", "");
    process.go();
    assert_eq!(process.read(), "locked");
    // An uncommitted trigger update must not notify other readers.
    let mut events = store.subscribe();
    while events.try_recv().is_ok() {}
    let server = StoreServer::spawn(store.clone());
    let mut doc = store.load_ledger(&id()).await.unwrap().unwrap();
    doc.add_user(
        NewUser {
            user_id: UserId::from_u128(3),
            display_name: "unsaved".into(),
        },
        Timestamp::from_millis(2000),
    )
    .unwrap();
    let started = std::time::Instant::now();
    assert!(server.save_ledger(&id(), &mut doc).await.is_err());
    assert!(started.elapsed() >= Duration::from_secs(4));
    assert_eq!(doc.list_users().unwrap().len(), 1);
    assert!(
        store
            .load_ledger(&id())
            .await
            .unwrap()
            .unwrap()
            .list_users()
            .unwrap()
            .is_empty()
    );
    while let Ok(event) = events.try_recv() {
        assert!(!matches!(event, ServiceEvent::DeviceLabelsUpdated));
    }
    process.child.kill().unwrap();
    let _ = process.child.wait();
    assert!(
        !store
            .list_device_labels()
            .await
            .unwrap()
            .contains_key("uncommitted")
    );
    server.save_ledger(&id(), &mut doc).await.unwrap();
    assert_eq!(
        store
            .load_ledger(&id())
            .await
            .unwrap()
            .unwrap()
            .list_users()
            .unwrap()
            .len(),
        1
    );
    let reopened = SqliteStore::open(dir.path().into()).await.unwrap();
    assert_eq!(
        reopened
            .load_ledger(&id())
            .await
            .unwrap()
            .unwrap()
            .list_users()
            .unwrap()
            .len(),
        1
    );
}

#[tokio::test]
async fn overlapping_writers_preserve_both_documents_and_independent_labels() {
    let dir = tempfile::tempdir().unwrap();
    let store = SqliteStore::open(dir.path().into()).await.unwrap();
    store.save_ledger(&id(), &mut document()).await.unwrap();
    let mut a = Process::spawn(dir.path(), "merge", "1");
    let mut b = Process::spawn(dir.path(), "merge", "2");
    a.go();
    b.go();
    assert_eq!(a.read(), "ready");
    assert_eq!(b.read(), "ready");
    a.go();
    b.go();
    assert_eq!(a.read(), "done");
    assert_eq!(b.read(), "done");
    a.finish();
    b.finish();
    assert_eq!(
        store
            .load_ledger(&id())
            .await
            .unwrap()
            .unwrap()
            .list_users()
            .unwrap()
            .len(),
        2
    );
    let labels = store.list_device_labels().await.unwrap();
    assert_eq!(labels["1"], "1");
    assert_eq!(labels["2"], "2");
}
