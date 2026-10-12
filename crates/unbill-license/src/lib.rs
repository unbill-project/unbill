//! Offline legal information embedded at compilation time.
//!
//! This is a conservative catalog of all normal workspace dependencies across
//! features and platforms, not a list of the crates linked into a particular
//! binary. It excludes dependencies used only for building or testing.
//! Non-Cargo native libraries and frontend assets need separate accounting.

mod types;

use std::sync::OnceLock;
pub use types::{LegalDictionary, LegalDocument, LicenseText, SoftwareLegalInfo, TextOrigin};

// sirno:witness:unbill-license:begin
/// Generated JSON, also suitable for tooling that wants to avoid Rust DTOs.
pub const LEGAL_INFORMATION_JSON: &str = include_str!(concat!(env!("OUT_DIR"), "/legal.json"));

/// Dictionary keyed by Cargo software name. Original texts are not paraphrased.
#[allow(
    clippy::expect_used,
    reason = "the build script serializes these exact DTOs"
)]
pub fn dictionary() -> &'static LegalDictionary {
    static DICTIONARY: OnceLock<LegalDictionary> = OnceLock::new();
    DICTIONARY.get_or_init(|| {
        serde_json::from_str(LEGAL_INFORMATION_JSON)
            .expect("build script must produce valid legal information")
    })
}

/// All version/source records for this software name, or an empty slice.
pub fn software(name: &str) -> &'static [SoftwareLegalInfo] {
    dictionary().get(name).map(Vec::as_slice).unwrap_or(&[])
}
// sirno:witness:unbill-license:end

#[cfg(test)]
#[path = "../build_support.rs"]
mod build_support;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_unbill_license_files_are_embedded_verbatim() {
        let records = software("unbill-license");
        let record = records
            .iter()
            .find(|r| r.version == env!("CARGO_PKG_VERSION"))
            .unwrap();
        assert_eq!(record.license_expression, "MIT OR Apache-2.0");
        assert!(
            record
                .documents
                .iter()
                .any(|d| d.text == include_str!("../../../LICENSE-MIT"))
        );
        assert!(
            record
                .documents
                .iter()
                .any(|d| d.text == include_str!("../../../LICENSE-APACHE"))
        );
    }

    #[test]
    fn catalog_contains_transitive_dependencies_and_no_empty_license_texts() {
        assert!(!software("iroh").is_empty());
        assert!(!software("serde").is_empty());
        for records in dictionary().values() {
            for record in records {
                assert!(!record.license_expression.is_empty());
                assert!(!record.license_texts.is_empty(), "{}", record.package_id);
                assert!(
                    record
                        .license_texts
                        .iter()
                        .all(|l| !l.text.trim().is_empty())
                );
            }
        }
        assert!(software("nonexistent-software").is_empty());
    }

    #[test]
    fn heuristic_scanning_cannot_add_undeclared_licenses() {
        for record in software("anyhow") {
            assert!(
                record
                    .license_texts
                    .iter()
                    .all(|text| { matches!(text.identifier.as_str(), "MIT" | "Apache-2.0") })
            );
            assert!(record.documents.iter().any(|document| {
                document.path == "LICENSE-APACHE" && document.text.contains("Apache License")
            }));
        }
    }
}
