use crate::types::{LegalDictionary, LegalDocument, SoftwareLegalInfo};
use std::{fs, io, path::Path};

/// Keep records distinct even when names and versions match but sources differ.
pub fn insert(dictionary: &mut LegalDictionary, info: SoftwareLegalInfo) {
    let records = dictionary.entry(info.name.clone()).or_default();
    records.push(info);
    records.sort_by(|a, b| a.package_id.cmp(&b.package_id));
}

pub fn documents(root: &Path, recursive: bool) -> io::Result<Vec<LegalDocument>> {
    let mut result = Vec::new();
    collect(root, root, recursive, &mut result)?;
    result.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(result)
}

fn collect(
    root: &Path,
    dir: &Path,
    recursive: bool,
    result: &mut Vec<LegalDocument>,
) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        let path = entry.path();
        if kind.is_dir() {
            if recursive
                && !matches!(
                    entry.file_name().to_str(),
                    Some(".git" | "target" | "node_modules")
                )
            {
                collect(root, &path, recursive, result)?;
            }
        } else if legal_file(&entry.file_name().to_string_lossy()) {
            // Do not silently follow links outside the package's source tree.
            if kind.is_symlink() && !fs::canonicalize(&path)?.starts_with(fs::canonicalize(root)?) {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("legal file escapes package: {}", path.display()),
                ));
            }
            result.push(LegalDocument {
                path: path
                    .strip_prefix(root)
                    .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?
                    .to_string_lossy()
                    .replace('\\', "/"),
                text: fs::read_to_string(path)?,
            });
        }
    }
    Ok(())
}

fn legal_file(name: &str) -> bool {
    if name.ends_with(".rs") || name.ends_with(".c") || name.ends_with(".h") {
        return false;
    }
    let upper = name.to_ascii_uppercase();
    [
        "LICENSE",
        "LICENCE",
        "COPYING",
        "NOTICE",
        "COPYRIGHT",
        "AUTHORS",
        "ATTRIBUTION",
        "PATENTS",
    ]
    .iter()
    .any(|prefix| {
        upper == *prefix
            || upper
                .strip_prefix(prefix)
                .is_some_and(|rest| rest.starts_with(['.', '-', '_']))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn original_notices_and_nested_vendor_attribution_are_preserved() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("vendor")).unwrap();
        fs::write(
            dir.path().join("LICENSE-MIT"),
            "Copyright Alice\nLicense text\n",
        )
        .unwrap();
        fs::write(
            dir.path().join("vendor/NOTICE"),
            "Bob's required attribution\n",
        )
        .unwrap();
        fs::write(dir.path().join("license.rs"), "// unrelated Rust source").unwrap();
        let files = documents(dir.path(), true).unwrap();
        assert_eq!(files.len(), 2);
        assert!(
            files
                .iter()
                .any(|f| f.path == "vendor/NOTICE" && f.text == "Bob's required attribution\n")
        );
        assert!(
            files
                .iter()
                .any(|f| f.path == "LICENSE-MIT" && f.text == "Copyright Alice\nLicense text\n")
        );
    }

    #[test]
    fn package_versions_and_distinct_sources_are_not_overwritten() {
        let mut catalog = LegalDictionary::new();
        for (version, id) in [
            ("1.0.0", "registry:a@1.0.0"),
            ("2.0.0", "registry:a@2.0.0"),
            ("1.0.0", "git:a@1.0.0"),
        ] {
            insert(
                &mut catalog,
                SoftwareLegalInfo {
                    name: "a".into(),
                    version: version.into(),
                    package_id: id.into(),
                    license_expression: "MIT".into(),
                    authors: vec![],
                    repository: None,
                    source: None,
                    source_archive: None,
                    license_texts: vec![],
                    documents: vec![],
                },
            );
        }
        assert_eq!(catalog["a"].len(), 3);
        assert_eq!(catalog["a"][0].package_id, "git:a@1.0.0");
    }
}
