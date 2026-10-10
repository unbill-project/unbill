//! Verify metadata belongs to the consumer and tracks its source changes.
use std::process::Command;

#[test]
fn application_owns_version_and_refreshes_timestamp_when_its_sources_change() {
    let fixture = tempfile::tempdir().unwrap();
    let root = fixture.path();
    std::fs::create_dir(root.join("src")).unwrap();
    let dependency = env!("CARGO_MANIFEST_DIR");
    std::fs::write(
        root.join("Cargo.toml"),
        format!(
            r#"
[workspace]
[package]
name = "build-info-consumer"
version = "9.8.7"
edition = "2024"
[dependencies]
unbill-build-info = {{ path = "{dependency}" }}
build-info = "=0.0.46"
[build-dependencies]
chrono = "0.4.45"
build-info-build = {{ version = "=0.0.46", default-features = false }}
"#
        ),
    )
    .unwrap();
    std::fs::write(
        root.join("build.rs"),
        "fn main() { build_info_build::build_script().build_timestamp(chrono::Utc::now()); }",
    )
    .unwrap();
    let source = "build_info::build_info!(fn compiled_build_info); fn main() { println!(\"{}\", unbill_build_info::BuildInfo::from(compiled_build_info())); }";
    std::fs::write(root.join("src/main.rs"), source).unwrap();
    let run = || {
        let output = Command::new(env!("CARGO"))
            .args(["run", "--quiet", "--offline"])
            .current_dir(root)
            .env("CARGO_TARGET_DIR", root.join("target"))
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap()
    };
    let first = run();
    assert!(first.starts_with("9.8.7 (built "));
    assert!(first.trim().ends_with("Z)"));
    assert_eq!(first, run(), "unchanged builds reuse their timestamp");
    std::thread::sleep(std::time::Duration::from_secs(1));
    std::fs::write(
        root.join("src/main.rs"),
        format!("{source}\n// changed consumer source\n"),
    )
    .unwrap();
    assert_ne!(
        first,
        run(),
        "consumer changes must refresh its own timestamp"
    );
}
