use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// One name may have several versions or sources in the dependency graph.
pub type LegalDictionary = BTreeMap<String, Vec<SoftwareLegalInfo>>;

/// Legal information for one Cargo package or the Unbill project.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SoftwareLegalInfo {
    pub name: String,
    pub version: String,
    pub package_id: String,
    pub license_expression: String,
    pub authors: Vec<String>,
    pub repository: Option<String>,
    /// Cargo's source identity, including the commit for Git dependencies.
    pub source: Option<String>,
    /// Exact crates.io archive URL for registry packages, if applicable.
    pub source_archive: Option<String>,
    pub license_texts: Vec<LicenseText>,
    /// Original license, NOTICE, COPYRIGHT, AUTHORS and attribution files.
    /// Copyright statements embedded in licenses are retained here verbatim.
    pub documents: Vec<LegalDocument>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LicenseText {
    /// SPDX license or exception identifier.
    pub identifier: String,
    pub text: String,
    pub origin: TextOrigin,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TextOrigin {
    /// Relative to the package root (or the workspace root for shared files).
    PackageFile { path: String },
    /// Standard text, not an assertion about the package's copyright holders.
    SpdxFallback,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LegalDocument {
    pub path: String,
    pub text: String,
}
