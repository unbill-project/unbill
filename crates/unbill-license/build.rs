mod build_support;
#[path = "src/types.rs"]
mod types;

use anyhow::{Context, Result, bail};
use cargo_about::licenses::{Gatherer, LicenseFileKind, LicenseInfo, config::Config};
use std::{env, fs, path::PathBuf, sync::Arc};
use types::{LegalDictionary, LicenseText, SoftwareLegalInfo, TextOrigin};

// sirno:witness:unbill-license:begin
fn main() -> Result<()> {
    let crate_root = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);
    let workspace = crate_root
        .parent()
        .and_then(|path| path.parent())
        .context("unbill-license must live under the workspace crates directory")?;
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=build_support.rs");
    println!("cargo:rerun-if-changed=src/types.rs");
    println!(
        "cargo:rerun-if-changed={}",
        workspace.join("Cargo.toml").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        workspace.join("Cargo.lock").display()
    );

    let config = Config {
        ignore_build_dependencies: true,
        ignore_dev_dependencies: true,
        ..Config::default()
    };
    let manifest = krates::Utf8PathBuf::from_path_buf(workspace.join("Cargo.toml"))
        .map_err(|_| anyhow::anyhow!("workspace path is not UTF-8"))?;
    let graph = cargo_about::get_all_crates(
        &manifest, false, true, vec![], true,
        krates::LockOptions { frozen: true, locked: true, offline: true },
        &config, &[],
    ).context("collecting locked workspace dependencies; run `cargo fetch --locked` before building offline")?;
    let store = Arc::new(cargo_about::licenses::store_from_cache()?);
    // Original notice files are collected separately, including nested vendors.
    let gathered = Gatherer::with_store(store)
        .with_max_depth(Some(1))
        .gather(&graph, &config, None);
    let shared = build_support::documents(workspace, false)?;
    for doc in &shared {
        println!(
            "cargo:rerun-if-changed={}",
            workspace.join(&doc.path).display()
        );
    }
    let mut dictionary = LegalDictionary::new();
    for info in gathered {
        let package = info.krate;
        let root = package
            .manifest_path
            .parent()
            .context("package manifest has no parent")?;
        println!("cargo:rerun-if-changed={}", package.manifest_path);
        let mut documents = build_support::documents(root.as_std_path(), true)
            .with_context(|| format!("reading legal documents for {}", package.id))?;
        for doc in &documents {
            println!("cargo:rerun-if-changed={}", root.join(&doc.path));
        }
        if package.source.is_none() && root.starts_with(workspace) {
            documents.extend(shared.iter().cloned().map(|mut doc| {
                doc.path = format!("workspace/{}", doc.path);
                doc
            }));
        }
        let LicenseInfo::Expr(expression) = &info.lic_info else {
            bail!(
                "unresolved license for {}; add verified license metadata before publishing",
                package.id
            );
        };
        let mut license_texts = Vec::new();
        for file in &info.license_files {
            let (LicenseFileKind::Text(text) | LicenseFileKind::AddendumText(text, _)) = &file.kind
            else {
                continue;
            };
            // Detection is heuristic. Do not advertise an undeclared license
            // based on a match; the original document is retained regardless.
            if !file.license_expr.requirements().all(|detected| {
                expression
                    .requirements()
                    .any(|declared| declared.req == detected.req)
            }) {
                continue;
            }
            let identifier = file.license_expr.to_string();
            license_texts.push(LicenseText {
                identifier,
                text: text.clone(),
                origin: TextOrigin::PackageFile {
                    path: file
                        .path
                        .strip_prefix(root)
                        .unwrap_or(&file.path)
                        .to_string(),
                },
            });
        }
        // Retain all OR alternatives and AND obligations, not just one selected license.
        for requirement in expression.requirements() {
            let Some(id) = requirement.req.license.id() else {
                bail!(
                    "custom license {} in {} requires an explicit text mapping",
                    requirement.req.license,
                    package.id
                );
            };
            if !license_texts.iter().any(|text| text.identifier == id.name) {
                let filename = if id.name == "Apache-2.0" {
                    "APACHE"
                } else {
                    id.name
                };
                let shared_file = documents.iter().find(|doc| {
                    doc.path == format!("workspace/LICENSE-{}", filename.to_ascii_uppercase())
                });
                license_texts.push(LicenseText {
                    identifier: id.name.to_owned(),
                    text: shared_file
                        .map(|doc| doc.text.clone())
                        .unwrap_or_else(|| id.text().to_owned()),
                    origin: shared_file
                        .map(|doc| TextOrigin::PackageFile {
                            path: doc.path.clone(),
                        })
                        .unwrap_or(TextOrigin::SpdxFallback),
                });
            }
            if let Some(addition) = &requirement.req.addition {
                let Some(exception) = addition.id() else {
                    bail!(
                        "custom exception {addition} in {} requires an explicit text mapping",
                        package.id
                    );
                };
                if !license_texts
                    .iter()
                    .any(|text| text.identifier == exception.name)
                {
                    license_texts.push(LicenseText {
                        identifier: exception.name.to_owned(),
                        text: exception.text().to_owned(),
                        origin: TextOrigin::SpdxFallback,
                    });
                }
            }
        }
        license_texts.sort_by(|a, b| (&a.identifier, &a.text).cmp(&(&b.identifier, &b.text)));
        license_texts.dedup();
        let source = package.source.as_ref().map(ToString::to_string);
        let source_archive = package
            .source
            .as_ref()
            .filter(|s| s.is_crates_io())
            .map(|_| {
                format!(
                    "https://crates.io/api/v1/crates/{}/{}/download",
                    package.name, package.version
                )
            });
        build_support::insert(
            &mut dictionary,
            SoftwareLegalInfo {
                name: package.name.to_string(),
                version: package.version.to_string(),
                package_id: format!(
                    "{}#{}@{}",
                    source.as_deref().unwrap_or("workspace"),
                    package.name,
                    package.version
                ),
                license_expression: expression.to_string(),
                authors: package.authors.clone(),
                repository: package.repository.clone(),
                source,
                source_archive,
                license_texts,
                documents,
            },
        );
    }
    let destination = PathBuf::from(env::var("OUT_DIR")?).join("legal.json");
    fs::write(destination, serde_json::to_vec(&dictionary)?)?;
    Ok(())
}
// sirno:witness:unbill-license:end
