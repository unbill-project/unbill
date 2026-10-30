---
core.name: Unbill License
core.desc: Compiled legal information for workspace software and its Rust dependencies.
core.category:
  - core.concept
core.belongs:
  - workspace-layout
---

`unbill-license` provides a read-only dictionary from software names to
versioned legal records. Records retain license texts, attribution and notice
files, package identity, and source locations. Multiple versions and sources of
a package remain distinct.

The `unbill` entry represents the project itself. It inherits the workspace
version, license expression, and repository, and embeds the original root
LICENSE, LICENSE-MIT, and LICENSE-APACHE files separately from crate entries.

The catalog is generated offline during compilation from the locked workspace
dependency graph and package sources using the cargo-about library. It includes
all features and platforms of normal workspace dependencies, not the exact
contents of one application binary. Build-only and development-only dependencies
are excluded. Source dependencies must be fetched before an offline build.

Original files are preserved verbatim. All SPDX license alternatives and
exceptions are retained; standard fallback texts are explicitly identified and
do not invent copyright holders. Unresolved package licenses fail generation.
Heuristic file matches cannot add undeclared licenses to the identified texts;
all original legal documents remain available even when detection disagrees.
Workspace crates retain the shared root license files.
The SPDX text scanner is optimized in development and release builds so the
catalog can be regenerated without scanning licenses in unoptimized code.

Runtime consumers need neither Cargo nor filesystem access. The same dictionary
and embedded JSON can serve command-line, terminal, GUI, and library consumers.
The CLI exposes the catalog and original legal documents through its offline
licenses command.
GUI consumers use compact software summaries and selected detail records.
Display helpers retain original documents first and add only distinct canonical
texts, so repeated license content does not obscure package-specific notices.
The catalog does not cover non-Cargo components, prove legal compliance, or
provide corresponding source itself.

The native Rust package shares the project's `unbill` name and workspace identity;
its workspace catalog record is represented by the project record with the original
root license texts. Records from different registry sources remain distinct.

Registry archives contain a generated compressed catalog snapshot. Repository
builds regenerate the catalog from the locked workspace; standalone builds copy
the packaged snapshot without requiring the repository or running Cargo metadata.
The Rust packaging command generates that snapshot before packaging and removes
its temporary source copy afterward.

The binary embeds only losslessly gzip-compressed catalog data. Repeated legal
text shares compression data without removing package-specific copyright or
NOTICE content. The dictionary is decoded once on first access; the full JSON
export is generated and cached only when legal_information_json() is requested.
LEGAL_INFORMATION_GZIP provides the compressed bytes for tooling.

Before compression, all identical legal texts are interned in a shared text table.
Package records retain their document paths and license origins and reference text
indices. Deduplication compares exact content, not SPDX identifiers: differing
copyright statements or NOTICE files remain separate.
