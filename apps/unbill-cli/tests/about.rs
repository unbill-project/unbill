use std::process::Command;

#[test]
fn about_is_available_without_a_daemon_or_writable_data_directory() {
    let dir = tempfile::tempdir().unwrap();
    let invalid_dir = dir.path().join("not-a-directory");
    std::fs::write(&invalid_dir, "sentinel").unwrap();
    for args in [vec!["about"], vec!["about", "--json"]] {
        let output = Command::new(env!("CARGO_BIN_EXE_unbill-cli"))
            .env("UNBILL_DATA_DIR", &invalid_dir)
            .env("UNBILL_SOCKET", dir.path().join("missing.sock"))
            .args(&args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(std::fs::read_to_string(&invalid_dir).unwrap(), "sentinel");
        if args.contains(&"--json") {
            let content: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
            assert!(content["sections"].as_array().unwrap().len() >= 3);
        } else {
            let text = String::from_utf8(output.stdout).unwrap();
            assert!(text.contains("https://github.com/unbill-project/unbill"));
        }
    }
}
