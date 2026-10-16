fn main() -> Result<(), Box<dyn std::error::Error>> {
    build_info_build::build_script().build_timestamp(chrono::Utc::now());
    println!("cargo:rerun-if-changed=macos-info.plist");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        let manifest = std::env::var("CARGO_MANIFEST_DIR")?;
        println!(
            "cargo:rustc-link-arg-bin=unbill-server=-Wl,-sectcreate,__TEXT,__info_plist,{manifest}/macos-info.plist"
        );
    }
    Ok(())
}
