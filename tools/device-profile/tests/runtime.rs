use anyhow::{Context as _, Result, ensure};
use serde_json::Value;
use std::{path::PathBuf, process::Command};

fn capture(store: &str) -> Result<()> {
    let root = std::env::temp_dir().join(format!(
        "unbill-runtime-test-{}-{store}",
        std::process::id()
    ));
    let output = Command::new(env!("CARGO_BIN_EXE_unbill-device-profile"))
        .args([
            "--store",
            store,
            "--scenario",
            "read",
            "--ledgers",
            "2",
            "--bills",
            "2",
            "--iterations",
            "2",
            "--concurrency",
            "4",
            "--output",
        ])
        .arg(&root)
        .output()?;
    ensure!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let result: Value = serde_json::from_slice(&std::fs::read(root.join("result.json"))?)?;
    ensure!(result["success"] == true, "capture failed");
    let rows: Vec<Value> = std::fs::read_to_string(root.join("spans.jsonl"))?
        .lines()
        .map(serde_json::from_str)
        .collect::<Result<_, _>>()?;
    ensure!(
        rows.iter().filter(|r| r["name"] == "workload.read").count() == 8,
        "missing workload spans"
    );
    let find = |id: &Value| rows.iter().find(|r| &r["id"] == id);
    for queue in rows.iter().filter(|r| r["name"] == "store.queue") {
        let request = find(&queue["parent"]).context("missing actor request parent")?;
        ensure!(request["name"] == "store.request", "wrong request parent");
        let execute = rows
            .iter()
            .find(|r| r["name"] == "store.execute" && r["parent"] == request["id"])
            .context("missing actor execution")?;
        let start = queue["start_us"].as_u64().context("queue start")?;
        let duration = queue["duration_us"].as_u64().context("queue duration")?;
        ensure!(
            execute["start_us"].as_u64().context("execution start")?
                >= start.saturating_add(duration),
            "queue overlaps execution"
        );
    }
    if store == "sqlite" {
        let execution = rows
            .iter()
            .find(|r| r["name"] == "sqlite.execute")
            .context("no SQLite execution")?;
        ensure!(
            find(&execution["parent"]).is_some(),
            "blocking thread lost its parent"
        );
    }
    // Capture text is an allowlist of labels/timings, even though the workload
    // authenticates and persists keys, sessions, user names and bill contents.
    let text = std::fs::read_to_string(root.join("spans.jsonl"))?;
    ensure!(
        !text.contains("synthetic-profile-password") && !text.contains("Synthetic bill"),
        "payload leaked into trace"
    );
    std::fs::remove_dir_all(root)?;
    Ok(())
}
#[test]
fn memory_runtime_keeps_actor_parentage() -> Result<()> {
    capture("memory")
}
#[test]
fn sqlite_runtime_keeps_blocking_parentage() -> Result<()> {
    capture("sqlite")
}

#[test]
fn timeout_preserves_a_failed_capture() -> Result<()> {
    let root: PathBuf =
        std::env::temp_dir().join(format!("unbill-runtime-timeout-{}", std::process::id()));
    let output = Command::new(env!("CARGO_BIN_EXE_unbill-device-profile"))
        .args([
            "--scenario",
            "idle",
            "--store",
            "memory",
            "--ledgers",
            "1",
            "--bills",
            "0",
            "--idle-secs",
            "30",
            "--timeout-secs",
            "1",
            "--output",
        ])
        .arg(&root)
        .output()?;
    ensure!(!output.status.success(), "timeout reported success");
    let result: Value = serde_json::from_slice(&std::fs::read(root.join("result.json"))?)?;
    ensure!(
        result["success"] == false && result["error"] == "workload timed out",
        "timeout result missing"
    );
    ensure!(
        root.join("spans.jsonl").is_file(),
        "partial capture missing"
    );
    std::fs::remove_dir_all(root)?;
    Ok(())
}
