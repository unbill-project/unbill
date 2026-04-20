// End-to-end tests for the unbill CLI.
//
// Each test gets an isolated temporary data directory via `Env`. Commands are
// run against the real compiled binary. Assertions use `--json` output.

use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Output, Stdio};
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// Test harness
// ---------------------------------------------------------------------------

struct Env {
    dir: TempDir,
}

impl Env {
    fn new() -> Self {
        Self {
            dir: tempfile::tempdir().unwrap(),
        }
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_unbill"))
            .env("UNBILL_DATA_DIR", self.dir.path())
            .args(args)
            .output()
            .expect("failed to spawn unbill")
    }

    /// Run a command, assert success, parse stdout as JSON.
    fn json(&self, args: &[&str]) -> serde_json::Value {
        let mut full = vec!["--json"];
        full.extend_from_slice(args);
        let out = self.run(&full);
        assert!(
            out.status.success(),
            "unbill {} failed\nstderr: {}",
            full.join(" "),
            String::from_utf8_lossy(&out.stderr),
        );
        serde_json::from_slice(&out.stdout).expect("stdout was not valid JSON")
    }

    /// Run a command, assert success.
    fn ok(&self, args: &[&str]) {
        let out = self.run(args);
        assert!(
            out.status.success(),
            "unbill {} failed\nstderr: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr),
        );
    }

    /// Run a command, assert failure.
    fn fail(&self, args: &[&str]) -> String {
        let out = self.run(args);
        assert!(
            !out.status.success(),
            "unbill {} was expected to fail but succeeded",
            args.join(" "),
        );
        String::from_utf8_lossy(&out.stderr).into_owned()
    }
}

// ---------------------------------------------------------------------------
// Daemon harness (for two-env peer tests)
// ---------------------------------------------------------------------------

/// A running `unbill sync daemon` child process.
///
/// `node_id` is the host's device ID, read from the first stdout line
/// (`"listening on: <node_id>"`). That line is printed only after the Iroh
/// endpoint is fully bound, so reading it also serves as a readiness signal.
///
/// The process is killed (and waited) when the guard is dropped.
struct Daemon {
    child: Child,
    /// The host's NodeId string — pass to `sync once` to dial this host.
    pub node_id: String,
}

impl Daemon {
    fn spawn(env: &Env) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_unbill"))
            .env("UNBILL_DATA_DIR", env.dir.path())
            .args(["sync", "daemon"])
            .stdout(Stdio::piped())
            .spawn()
            .expect("failed to spawn daemon");
        let stdout = child.stdout.take().unwrap();
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        reader
            .read_line(&mut line)
            .expect("failed to read daemon stdout");
        let node_id = line
            .strip_prefix("listening on: ")
            .unwrap_or_else(|| panic!("unexpected daemon output: {line:?}"))
            .trim()
            .to_string();
        Daemon { child, node_id }
    }
}

impl Drop for Daemon {
    fn drop(&mut self) {
        self.child.kill().ok();
        self.child.wait().ok();
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const ALICE: &str = "00000000000000000000000001";
const BOB: &str = "00000000000000000000000002";
const CAROL: &str = "00000000000000000000000003";

fn create_ledger(env: &Env) -> String {
    let v = env.json(&["ledger", "create", "Household", "USD"]);
    v["ledger_id"].as_str().unwrap().to_owned()
}

fn add_user(env: &Env, ledger_id: &str, user_id: &str, name: &str) {
    env.ok(&[
        "user",
        "add",
        "--ledger-id",
        ledger_id,
        "--user-id",
        user_id,
        "--name",
        name,
    ]);
}

/// Create a ledger and register ALICE, BOB, and CAROL as users.
fn create_ledger_with_users(env: &Env) -> String {
    let lid = create_ledger(env);
    add_user(env, &lid, ALICE, "Alice");
    add_user(env, &lid, BOB, "Bob");
    add_user(env, &lid, CAROL, "Carol");
    lid
}

fn add_bill(
    env: &Env,
    ledger_id: &str,
    payer: &str,
    amount: &str,
    desc: &str,
    share_users: &[&str],
) -> String {
    let mut args = vec![
        "bill",
        "add",
        "--ledger-id",
        ledger_id,
        "--payer",
        payer,
        "--amount",
        amount,
        "--description",
        desc,
    ];
    for user_id in share_users {
        args.push("--share-user");
        args.push(user_id);
    }
    let v = env.json(&args);
    v["bill_id"].as_str().unwrap().to_owned()
}

// ---------------------------------------------------------------------------
// Device init
// ---------------------------------------------------------------------------

#[test]
fn test_init_prints_device_id() {
    let env = Env::new();
    let v = env.json(&["init"]);
    let id = v["device_id"].as_str().unwrap();
    assert_eq!(id.len(), 64, "device ID should be 64 hex chars");
}

#[test]
fn test_device_id_is_stable_across_calls() {
    let env = Env::new();
    let id1 = env.json(&["init"])["device_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let id2 = env.json(&["init"])["device_id"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(id1, id2);
}

// ---------------------------------------------------------------------------
// Device
// ---------------------------------------------------------------------------

#[test]
fn test_device_show_returns_id_and_data_dir() {
    let env = Env::new();
    let v = env.json(&["device", "show"]);
    assert_eq!(v["device_id"].as_str().unwrap().len(), 64);
    assert!(
        v["data_dir"].as_str().unwrap().contains("unbill")
            || !v["data_dir"].as_str().unwrap().is_empty()
    );
}

#[test]
fn test_device_show_id_matches_init() {
    let env = Env::new();
    let init_id = env.json(&["init"])["device_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let show_id = env.json(&["device", "show"])["device_id"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(init_id, show_id);
}

// ---------------------------------------------------------------------------
// Ledger lifecycle
// ---------------------------------------------------------------------------

#[test]
fn test_create_ledger_returns_id() {
    let env = Env::new();
    let v = env.json(&["ledger", "create", "Trip", "EUR"]);
    let id = v["ledger_id"].as_str().unwrap();
    assert_eq!(id.len(), 26, "ledger ID should be a 26-char ULID");
}

#[test]
fn test_created_ledger_appears_in_list() {
    let env = Env::new();
    let id = create_ledger(&env);
    let list = env.json(&["ledger", "list"]);
    let ledgers = list.as_array().unwrap();
    assert_eq!(ledgers.len(), 1);
    assert_eq!(ledgers[0]["id"].as_str().unwrap(), id);
    assert_eq!(ledgers[0]["name"].as_str().unwrap(), "Household");
    assert_eq!(ledgers[0]["currency"].as_str().unwrap(), "USD");
}

#[test]
fn test_ledger_show_reports_bill_and_user_counts() {
    let env = Env::new();
    let lid = create_ledger_with_users(&env);
    add_bill(&env, &lid, ALICE, "30", "Dinner", &[ALICE, BOB]);
    let v = env.json(&["ledger", "show", &lid]);
    assert_eq!(v["bill_count"].as_u64().unwrap(), 1);
    assert_eq!(v["user_count"].as_u64().unwrap(), 3);
}

#[test]
fn test_delete_ledger_removes_it_from_list() {
    let env = Env::new();
    let id = create_ledger(&env);
    env.ok(&["ledger", "delete", &id]);
    let list = env.json(&["ledger", "list"]);
    assert!(list.as_array().unwrap().is_empty());
}

#[test]
fn test_invalid_currency_is_rejected() {
    let env = Env::new();
    let stderr = env.fail(&["ledger", "create", "Bad", "ZZZ"]);
    assert!(
        stderr.contains("currency"),
        "expected error about currency, got: {stderr}"
    );
}

// ---------------------------------------------------------------------------
// Bills
// ---------------------------------------------------------------------------

#[test]
fn test_add_bill_returns_id_and_appears_in_list() {
    let env = Env::new();
    let lid = create_ledger_with_users(&env);
    let bid = add_bill(&env, &lid, ALICE, "45.50", "Groceries", &[ALICE, BOB]);
    assert_eq!(bid.len(), 26);

    let bills = env.json(&["bill", "list", "--ledger-id", &lid]);
    let arr = bills.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["id"].as_str().unwrap(), bid);
    assert_eq!(arr[0]["amount_cents"].as_i64().unwrap(), 4550);
    assert_eq!(arr[0]["description"].as_str().unwrap(), "Groceries");
    assert!(arr[0]["prev"].as_array().unwrap().is_empty());
}

#[test]
fn test_amend_bill_supersedes_original() {
    let env = Env::new();
    let lid = create_ledger_with_users(&env);
    let bid = add_bill(&env, &lid, ALICE, "10", "Lunch", &[ALICE, BOB]);

    let amended = env.json(&[
        "bill",
        "amend",
        "--ledger-id",
        &lid,
        "--prev",
        &bid,
        "--payer",
        ALICE,
        "--amount",
        "12.50",
        "--description",
        "Lunch + coffee",
        "--share-user",
        ALICE,
        "--share-user",
        BOB,
    ]);
    let new_bid = amended["bill_id"].as_str().unwrap();
    assert_ne!(new_bid, bid, "amendment must have a new ID");

    let bills = env.json(&["bill", "list", "--ledger-id", &lid]);
    let arr = bills.as_array().unwrap();
    assert_eq!(arr.len(), 1, "only the amendment should be effective");
    let b = &arr[0];
    assert_eq!(b["id"].as_str().unwrap(), new_bid);
    assert_eq!(b["amount_cents"].as_i64().unwrap(), 1250);
    assert_eq!(b["description"].as_str().unwrap(), "Lunch + coffee");
    assert_eq!(b["prev"].as_array().unwrap()[0].as_str().unwrap(), bid);
}

// ---------------------------------------------------------------------------
// Settlement
// ---------------------------------------------------------------------------

#[test]
fn test_settlement_empty_when_no_bills() {
    let env = Env::new();
    create_ledger_with_users(&env);
    // Alice is a user but there are no bills — no transactions.
    let v = env.json(&["settlement", ALICE]);
    assert!(v["transactions"].as_array().unwrap().is_empty());
}

#[test]
fn test_settlement_correct_after_one_bill() {
    let env = Env::new();
    let lid = create_ledger_with_users(&env);
    // Alice pays $90, split equally with Bob — Bob owes Alice $45.
    add_bill(&env, &lid, ALICE, "90", "Dinner", &[ALICE, BOB]);

    let v = env.json(&["settlement", ALICE]);
    let txns = v["transactions"].as_array().unwrap();
    assert_eq!(txns.len(), 1);
    assert_eq!(txns[0]["from_user_id"].as_str().unwrap(), BOB);
    assert_eq!(txns[0]["to_user_id"].as_str().unwrap(), ALICE);
    assert_eq!(txns[0]["amount_cents"].as_i64().unwrap(), 4500);
}

#[test]
fn test_settlement_net_of_multiple_bills() {
    let env = Env::new();
    let lid = create_ledger_with_users(&env);
    // Alice pays $60 for Alice+Bob: Bob owes $30.
    add_bill(&env, &lid, ALICE, "60", "Rent", &[ALICE, BOB]);
    // Bob pays $30 for Alice+Bob: Alice owes $15. Net: Bob owes $15 to Alice.
    add_bill(&env, &lid, BOB, "30", "Utilities", &[ALICE, BOB]);

    let v = env.json(&["settlement", ALICE]);
    let txns = v["transactions"].as_array().unwrap();
    assert_eq!(txns.len(), 1);
    assert_eq!(txns[0]["amount_cents"].as_i64().unwrap(), 1500);
}

#[test]
fn test_settlement_aggregates_across_ledgers() {
    let env = Env::new();

    // Ledger 1: Alice pays $60, Alice+Bob split → Bob owes Alice $30.
    let lid1 = create_ledger_with_users(&env);
    add_bill(&env, &lid1, ALICE, "60", "Rent", &[ALICE, BOB]);

    // Ledger 2: Bob pays $20, Alice+Bob split → Alice owes Bob $10. Net: Bob owes Alice $20.
    let lid2 = create_ledger_with_users(&env);
    add_bill(&env, &lid2, BOB, "20", "Utilities", &[ALICE, BOB]);

    let v = env.json(&["settlement", ALICE]);
    let txns = v["transactions"].as_array().unwrap();
    assert_eq!(txns.len(), 1);
    assert_eq!(txns[0]["from_user_id"].as_str().unwrap(), BOB);
    assert_eq!(txns[0]["to_user_id"].as_str().unwrap(), ALICE);
    assert_eq!(txns[0]["amount_cents"].as_i64().unwrap(), 2000);
}

// ---------------------------------------------------------------------------
// Persistence
// ---------------------------------------------------------------------------

#[test]
fn test_data_persists_across_process_restarts() {
    let env = Env::new();
    let lid = create_ledger_with_users(&env);
    let bid = add_bill(&env, &lid, ALICE, "50", "Pizza", &[ALICE, BOB, CAROL]);

    // New process, same data dir.
    let bills = env.json(&["bill", "list", "--ledger-id", &lid]);
    let arr = bills.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["id"].as_str().unwrap(), bid);
    assert_eq!(arr[0]["amount_cents"].as_i64().unwrap(), 5000);
}

#[test]
fn test_amendments_persist_across_process_restarts() {
    let env = Env::new();
    let lid = create_ledger_with_users(&env);
    let bid = add_bill(&env, &lid, ALICE, "10", "Coffee", &[ALICE, BOB]);
    env.ok(&[
        "bill",
        "amend",
        "--ledger-id",
        &lid,
        "--prev",
        &bid,
        "--payer",
        ALICE,
        "--amount",
        "15",
        "--description",
        "Coffee",
        "--share-user",
        ALICE,
        "--share-user",
        BOB,
    ]);

    // New process reads the amended value.
    let bills = env.json(&["bill", "list", "--ledger-id", &lid]);
    let arr = bills.as_array().unwrap();
    assert_eq!(arr.len(), 1, "only the amendment should be effective");
    assert_eq!(arr[0]["amount_cents"].as_i64().unwrap(), 1500);
    assert_eq!(arr[0]["prev"].as_array().unwrap()[0].as_str().unwrap(), bid);
}

// ---------------------------------------------------------------------------
// Error cases
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Users
// ---------------------------------------------------------------------------

#[test]
fn test_user_add_appears_in_list() {
    let env = Env::new();
    let lid = create_ledger(&env);
    add_user(&env, &lid, ALICE, "Alice");
    let users = env.json(&["user", "list", "--ledger-id", &lid]);
    let arr = users.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["user_id"].as_str().unwrap(), ALICE);
    assert_eq!(arr[0]["display_name"].as_str().unwrap(), "Alice");
}

#[test]
fn test_add_bill_rejects_non_user() {
    let env = Env::new();
    let lid = create_ledger(&env);
    add_user(&env, &lid, ALICE, "Alice");
    // BOB is not a user in the ledger — bill should fail.
    let stderr = env.fail(&[
        "bill",
        "add",
        "--ledger-id",
        &lid,
        "--payer",
        ALICE,
        "--amount",
        "10",
        "--description",
        "Test",
        "--share-user",
        ALICE,
        "--share-user",
        BOB,
    ]);
    assert!(
        stderr.contains("not in this ledger"),
        "expected 'not in this ledger' error, got: {stderr}"
    );
}

// ---------------------------------------------------------------------------
// Error cases
// ---------------------------------------------------------------------------

#[test]
fn test_bill_list_on_unknown_ledger_fails() {
    let env = Env::new();
    env.fail(&["bill", "list", "--ledger-id", "00000000000000000000000000"]);
}

#[test]
fn test_m3_commands_are_not_yet_available() {
    let env = Env::new();
    let stderr = env.fail(&["sync", "status"]);
    assert!(stderr.contains("M3"), "expected M3 message, got: {stderr}");
}

// ---------------------------------------------------------------------------
// Two-env peer tests (join, sync, identity import)
// ---------------------------------------------------------------------------

/// Host creates a ledger and generates an invite URL; joiner calls
/// `ledger join`; after the daemon stops, the joiner's ledger list
/// contains exactly the host's ledger.
#[test]
fn test_join_flow() {
    let host = Env::new();
    let joiner = Env::new();

    let lid = create_ledger(&host);
    add_user(&host, &lid, ALICE, "Alice");

    let invite = host.json(&["ledger", "invite", &lid]);
    let url = invite["url"].as_str().unwrap().to_owned();

    let daemon = Daemon::spawn(&host);
    joiner.ok(&["ledger", "join", &url, "--label", "joiner"]);
    drop(daemon);

    let ledgers = joiner.json(&["ledger", "list"]);
    let arr = ledgers.as_array().unwrap();
    assert_eq!(arr.len(), 1, "joiner should have exactly one ledger");
    assert_eq!(arr[0]["id"].as_str().unwrap(), lid);
}

/// After joining, the host adds a bill. The joiner runs `sync once` against
/// the host daemon and then sees the new bill.
#[test]
fn test_sync_once_propagates_bills() {
    let host = Env::new();
    let joiner = Env::new();

    // Set up: joiner joins host's ledger.
    let lid = create_ledger_with_users(&host);
    let invite = host.json(&["ledger", "invite", &lid]);
    let url = invite["url"].as_str().unwrap().to_owned();
    let daemon = Daemon::spawn(&host);
    joiner.ok(&["ledger", "join", &url, "--label", "joiner"]);
    drop(daemon);

    // Host adds a bill while the joiner is offline.
    add_bill(&host, &lid, ALICE, "30.00", "Dinner", &[ALICE, BOB, CAROL]);

    // Joiner syncs.
    let daemon = Daemon::spawn(&host);
    joiner.ok(&["sync", "once", &daemon.node_id]);
    drop(daemon);

    // Joiner now sees the bill.
    let bills = joiner.json(&["bill", "list", "--ledger-id", &lid]);
    assert_eq!(
        bills.as_array().unwrap().len(),
        1,
        "joiner should see the host's bill after sync"
    );
}

/// Host generates an identity-share URL; the joiner runs `identity import`;
/// the imported identity then appears in the joiner's identity list.
#[test]
fn test_identity_import_flow() {
    let host = Env::new();
    let joiner = Env::new();

    let identity = host.json(&["identity", "create", "Alice"]);
    let user_id = identity["user_id"].as_str().unwrap().to_owned();

    let share = host.json(&["identity", "share", "--user-id", &user_id]);
    let url = share["url"].as_str().unwrap().to_owned();

    let daemon = Daemon::spawn(&host);
    joiner.ok(&["identity", "import", &url]);
    drop(daemon);

    let identities = joiner.json(&["identity", "list"]);
    let arr = identities.as_array().unwrap();
    assert_eq!(arr.len(), 1, "joiner should have one imported identity");
    assert_eq!(arr[0]["user_id"].as_str().unwrap(), user_id);
    assert_eq!(arr[0]["display_name"].as_str().unwrap(), "Alice");
}
