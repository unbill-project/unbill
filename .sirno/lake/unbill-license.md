---
core.name: Unbill License
core.desc: Compiled legal information for workspace software and its Rust dependencies.
core.category:
  - core.concept
core.belongs:
  - workspace-layout
---

`unbill-license` provides a read-only dictionary from Cargo package names to
versioned legal records. Records retain license texts, attribution and notice
files, package identity, and source locations. Multiple versions and sources of
a package remain distinct.

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
The catalog does not cover non-Cargo components, prove legal compliance, or
provide corresponding source itself.
