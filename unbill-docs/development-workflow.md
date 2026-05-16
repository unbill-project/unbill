---
name: Development Workflow
desc: The design-first, test-first, and documentation discipline for changing Unbill.
category:
  - meta
belongs:
  - unbill
---

Non-trivial functionality starts from design,
then a failing test,
then implementation,
then refactoring with tests as the safety net.
No production code ships without a prior failing test,
except type definitions, `todo!` stubs, and module declarations.

Tests are written before or alongside implementation.
Rust unit tests live in `#[cfg(test)]` modules at the bottom of the file they cover.
Test names describe behavior rather than implementation details.

Test priority follows risk and locality:
pure functions first,
then storage round trips with `InMemoryStore`,
then CRDT convergence with `proptest`,
then sync protocol tests over in-process channels,
then CLI end-to-end tests against real temporary directories.

Documentation must describe the current intended design and current implementation.
It should not keep history for discarded approaches.
When an open question is resolved,
the answer is folded into the relevant entry and the question is removed.

Docs and code change together.
Drift between design and implementation is worse than missing documentation.
Documentation prose stays conceptual and avoids embedding code that will drift.

Each crate needs design and implementation coverage before substantial implementation begins.
Submodules that own a significant design surface need their own coverage too,
such as storage, networking, settlement, document projection, conflict detection, and service orchestration.

This Sirno lake is the structured design source for reorganized project knowledge.
Compatibility Markdown files may remain as reader entry points,
but durable design facts should be represented as Sirno entries.

---

> **Sirno generated links begin. Do not edit this section.**

- belongs (to):
  - [unbill](unbill.md)
- belongs (from):
  - [compiled-markdown-artifacts](compiled-markdown-artifacts.md)
  - [documentation-coverage](documentation-coverage.md)

> **Sirno generated links end.**
