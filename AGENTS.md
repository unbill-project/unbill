# Unbill Repository Instructions

<!-- Compiled from unbill-docs. Update the Sirno lake first, then refresh this artifact. -->

<!-- sirno:witness:compiled-markdown-artifacts:begin -->

The structured design source lives in [unbill-docs](unbill-docs).
Start with [unbill-docs/introduction.md](unbill-docs/introduction.md).
Use [unbill-docs/documentation-coverage.md](unbill-docs/documentation-coverage.md) to find
where old documentation facts live.

<!-- sirno:witness:compiled-markdown-artifacts:end -->

## Design-first Development

<!-- sirno:witness:development-workflow:begin -->

Non-trivial functionality starts from design,
then a failing test,
then implementation,
then refactoring with tests as the safety net.
No production code ships without a prior failing test,
except type definitions, `todo!` stubs, and module declarations.

Write or update the relevant Sirno entry before production code.
Root Markdown files are compiled artifacts,
not separate design authority.

## Test-first Development

Tests are written before or alongside implementation.
Rust unit tests live in `#[cfg(test)]` modules at the bottom of the file they cover.
Test names describe behavior rather than implementation details.

`prek` is the local lint and formatting gate.
Before handing work back,
run `prek run --all-files` when the change touches tracked source,
documentation,
configuration,
workflows,
or release packaging.
If `prek` is not installed,
install it with `cargo install prek` or run the equivalent checks from `prek.toml`.
The `mdformat` hook intentionally skips `unbill-docs`;
use Sirno checks for lake structure and generated links.
Use targeted tests in addition to `prek` for behavior changes.

Test priority follows risk and locality:

1. Pure functions with no I/O.
1. Storage round trips with `InMemoryStore`.
1. CRDT convergence with `proptest`.
1. Sync protocol tests over in-process channels.
1. CLI end-to-end tests against real temporary directories.

## Documentation Rules

Documentation describes current intended design and current implementation.
Do not keep history for discarded approaches.
When an open question is resolved,
fold the answer into the relevant entry and remove the question.

Docs and code change together.
Drift between design and implementation is worse than missing documentation.
Documentation prose stays conceptual and avoids embedding code that will drift.

Each crate and significant submodule needs Sirno coverage before substantial implementation begins.
Submodules with design surface include storage, networking, settlement,
document projection, conflict detection, and service orchestration.

<!-- sirno:witness:development-workflow:end -->
