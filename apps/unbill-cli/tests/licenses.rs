use std::process::{Command, Output};

fn run(args: &[&str]) -> (tempfile::TempDir, Output) {
    let dir = tempfile::tempdir().unwrap();
    // A file cannot be a data directory. This also proves the command does not
    // try to initialize local state before displaying embedded information.
    let invalid_dir = dir.path().join("not-a-directory");
    std::fs::write(&invalid_dir, "sentinel").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_unbill-cli"))
        .env("UNBILL_DATA_DIR", invalid_dir)
        .env("UNBILL_SOCKET", dir.path().join("missing.sock"))
        .args(args)
        .output()
        .unwrap();
    (dir, output)
}

#[test]
fn licenses_are_available_without_a_daemon_or_data_directory() {
    let (_dir, output) = run(&["licenses"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("Rust workspace legal catalog"));
    assert!(text.contains("unbill-cli"));
    assert!(text.contains("licenses <software>"));
}

#[test]
fn selected_software_includes_original_license_text_and_json_records() {
    let (_dir, output) = run(&["licenses", "unbill-license"]);
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("Permission is hereby granted"));
    assert!(text.contains("Apache License"));

    let (_dir, output) = run(&["--json", "licenses", "unbill-license"]);
    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json.as_object().unwrap().len(), 1);
    assert_eq!(
        json["unbill-license"][0]["license_expression"],
        "MIT OR Apache-2.0"
    );
}

#[test]
fn unknown_software_fails_without_trying_to_connect_to_daemon() {
    let (_dir, output) = run(&["licenses", "does-not-exist"]);
    assert!(!output.status.success());
    let error = String::from_utf8(output.stderr).unwrap();
    assert!(error.contains("unknown software"));
    assert!(!error.contains("cannot connect"));
}
