use crate::{LegalDocument, SoftwareLegalInfo, TextOrigin, dictionary};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SoftwareSummary {
    pub name: String,
    pub versions: Vec<String>,
    pub license_expressions: Vec<String>,
}

/// Lightweight list for GUI selectors, with Unbill before third-party software.
pub fn software_summaries() -> Vec<SoftwareSummary> {
    let mut summaries: Vec<_> = dictionary()
        .iter()
        .map(|(name, records)| SoftwareSummary {
            name: name.clone(),
            versions: records
                .iter()
                .map(|r| r.version.clone())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect(),
            license_expressions: records
                .iter()
                .map(|r| r.license_expression.clone())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect(),
        })
        .collect();
    summaries.sort_by(|a, b| (a.name != "unbill", &a.name).cmp(&(b.name != "unbill", &b.name)));
    summaries
}

/// Original documents first, followed by distinct identified license texts.
/// Copyright and attribution are not replaced by generic SPDX templates.
pub fn display_documents(package: &SoftwareLegalInfo) -> Vec<LegalDocument> {
    let mut seen = BTreeSet::new();
    let mut documents = Vec::new();
    for document in &package.documents {
        if seen.insert(document.text.as_str()) {
            documents.push(document.clone());
        }
    }
    for license in &package.license_texts {
        if seen.insert(license.text.as_str()) {
            let source = match &license.origin {
                TextOrigin::PackageFile { path } => path.as_str(),
                TextOrigin::SpdxFallback => "standard SPDX text",
            };
            documents.push(LegalDocument {
                path: format!("{} — {source}", license.identifier),
                text: license.text.clone(),
            });
        }
    }
    documents
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LicenseText, software};

    #[test]
    fn software_selector_starts_with_unbill_and_preserves_version_choices() {
        let summaries = software_summaries();
        assert_eq!(summaries.first().unwrap().name, "unbill");
        let serde = summaries.iter().find(|s| s.name == "serde").unwrap();
        for record in software("serde") {
            assert!(serde.versions.contains(&record.version));
            assert!(
                serde
                    .license_expressions
                    .contains(&record.license_expression)
            );
        }
    }

    #[test]
    fn copyright_and_notice_texts_survive_deduplication() {
        let mut package = software("unbill")[0].clone();
        package.documents = vec![
            LegalDocument {
                path: "LICENSE".into(),
                text: "Copyright Alice\nTerms".into(),
            },
            LegalDocument {
                path: "NOTICE".into(),
                text: "Credit Bob".into(),
            },
        ];
        package.license_texts = vec![
            LicenseText {
                identifier: "MIT".into(),
                text: "Copyright Alice\nTerms".into(),
                origin: TextOrigin::PackageFile {
                    path: "LICENSE".into(),
                },
            },
            LicenseText {
                identifier: "MIT".into(),
                text: "Standard terms".into(),
                origin: TextOrigin::SpdxFallback,
            },
        ];
        let documents = display_documents(&package);
        assert_eq!(documents.len(), 3);
        assert_eq!(documents[0].text, "Copyright Alice\nTerms");
        assert_eq!(documents[1].path, "NOTICE");
        assert_eq!(documents[2].path, "MIT — standard SPDX text");
    }
}
