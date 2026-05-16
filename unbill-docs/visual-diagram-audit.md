---
name: Visual Diagram Audit
desc: The recovered diagram inventory and correctness notes for the Sirno migration.
category:
  - meta
belongs:
  - documentation-coverage
---

This entry records the visual diagrams recovered from the deleted Markdown documentation
and checked against the current repository.

Recovered as current:
`unbill` contains the system view.
`shared-and-local-state` contains the shared/local state boundary.
`bill-amendment` contains the supersession graph.
`conflict-detection` contains the competing-amendment graph.
`sync-behavior` contains device-to-UI event propagation.
`ui-shared-model` contains screen navigation.
`unbill-console` contains the console module relationship diagram.
`console-service` contains the service orchestration flow.
`ledger-doc` contains the Automerge boundary flow.
`settlement` contains the ledger-to-balances-to-transactions flow.
`unbill-storage` contains the storage data boundary.
`symmetric-channel` contains sync and join sequence diagrams.
`fs-store` contains the filesystem layout.

Recovered with corrections:
`server-http-api` replaces the old storage REST diagram with the current `HttpAsymChannel`
to `unbill-server` route shape.
`unbill-tui` restores the crate structure diagram and includes current `mod.rs` nodes.
`ui-components` restores the component module structure and adds the current `progress.rs` module.
`ci-cd-pipeline` replaces stale GitHub workflow diagrams with current Homebrew, Docker,
mobile, daemon, and package-track relationships.

Not recovered as diagrams:
Shell command examples, JSON examples, Rust trait sketches, and state-field listings from old docs
were treated as code or examples rather than visual diagrams.
Their durable design facts are covered by the relevant entries.

---

> **Sirno generated links begin. Do not edit this section.**

- belongs (to):
  - [documentation-coverage](documentation-coverage.md)
- belongs (from): (none)

> **Sirno generated links end.**
