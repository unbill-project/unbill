# unbill-license

Compile-time legal information for Unbill's Rust software. No Cargo, network, or
source files are needed at runtime.

```rust
let dictionary = unbill_license::dictionary();
for package in unbill_license::software("iroh") {
    println!("{} {}: {}", package.name, package.version, package.license_expression);
    for license in &package.license_texts {
        println!("{}\n{}", license.identifier, license.text);
    }
    for document in &package.documents {
        println!("{}\n{}", document.path, document.text);
    }
}
```

`dictionary()` returns a `BTreeMap<String, Vec<SoftwareLegalInfo>>`. The vector
preserves multiple versions and sources with the same software name. Records
contain SPDX expressions, full license texts and exceptions, original license
and attribution documents, package authors, repository and Cargo source identity,
and exact crates.io source archive URLs. `LEGAL_INFORMATION_JSON` exposes the
same embedded data for other consumers.

The `unbill` entry contains the project's own original root license files,
workspace version, license expression, and repository.

The build script uses cargo-about's library API; no separate cargo-about command
installation is required. It collects the locked workspace's normal dependency
graph with all features and platforms. This is a conservative workspace catalog,
**not an exact list of components in any particular binary**. Dependencies used
only for builds or tests are excluded. Fetch package sources before an offline
build:

```sh
cargo fetch --locked
cargo test --locked -p unbill-license
```

Original LICENSE, COPYING, NOTICE, COPYRIGHT, AUTHORS, ATTRIBUTION and PATENTS
files are retained, including nested vendored documents. Shared workspace license
files are included for local packages. Canonical SPDX fallback texts are marked
`SpdxFallback`; they do not replace package-specific copyright attribution.
Missing or unsupported license metadata fails the build instead of silently
dropping a package. Unknown/custom licenses need a reviewed text mapping before
they can be supported.

The catalog does not collect JavaScript assets, system libraries or every native
component used by a build script, and does not certify legal compliance. Preserve
and display relevant original documents as well as license texts. Corresponding
source availability, modification notices and application-specific legal notices
remain responsibilities of each distributor.

The CLI exposes the embedded catalog without connecting to a daemon:

```sh
unbill-cli licenses                       # List software names, versions and licenses
unbill-cli licenses unbill                # Unbill's own original license texts
unbill-cli licenses unbill-license        # Full license texts and original notices
unbill-cli licenses --all                 # All full texts and notices
unbill-cli --json licenses iroh           # Selected software records as JSON
```
