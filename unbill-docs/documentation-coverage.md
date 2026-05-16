---
name: Documentation Coverage
desc: The map from existing repository documentation to reorganized Sirno entries.
category:
  - meta
belongs:
  - development-workflow
---

This entry records where the pre-Sirno Markdown documentation is represented in the lake.
Each old Markdown file outside `unbill-docs` was checked before deletion.
Visual diagrams were recovered and checked in `visual-diagram-audit`.
`AGENTS.md` and `README.md` were then restored as compiled artifacts from `compiled-markdown-artifacts`.

Root documentation:
`AGENTS.md` is represented by `development-workflow`.
`README.md` is represented by `unbill`, `project-boundaries`,
`workspace-layout`, `distribution-and-release`, `ci-cd-pipeline`,
and `development-workflow`.
Root `DESIGN.md` is represented by `design-principles`, `device-console-split`,
`channels`, `data-model`, `users-and-devices`, `shared-and-local-state`,
`bill-amendment`, `conflict-detection`, `sync-behavior`,
`deployment-topologies`, `security-model`, and `project-boundaries`.
Root `IMPLEMENTATION.md` is represented by `workspace-layout`,
`unbill-model`, `unbill-storage`, `fs-store`, `memory-store`,
`unbill-event`, `symmetric-channel`, `asymmetric-channel`,
`unbill-device`, `unbill-console`, `ui-components`,
and the application entries.

CI/CD documentation:
`.github/DESIGN.md` and `.github/IMPLEMENTATION.md` are represented by `ci-cd-pipeline`
and `distribution-and-release`.

Application documentation:
`apps/DESIGN.md` is represented by `ui-shared-model`.
`apps/unbill-cli` docs are represented by `unbill-cli`.
`apps/unbill-daemon` docs are represented by `unbill-daemon`.
`apps/unbill-server` docs are represented by `unbill-server` and `server-http-api`.
`apps/unbill-tui` docs are represented by `unbill-tui`.
`apps/unbill-ui-native` docs are represented by `unbill-ui-native` and `ui-shared-model`.
`apps/unbill-ui-remote` docs are represented by `unbill-ui-remote`,
`server-http-api`, and `ui-shared-model`.

Crate documentation:
`crates/unbill-model` docs are represented by `unbill-model`.
`crates/unbill-storage` docs are represented by `unbill-storage` and `ledger-doc`.
`crates/unbill-store-fs` docs are represented by `fs-store`.
`crates/unbill-store-memory` docs are represented by `memory-store`.
`crates/unbill-event` docs are represented by `unbill-event`.
`crates/unbill-symmetric-channel` docs are represented by `symmetric-channel`.
`crates/unbill-asymmetric-channel` docs are represented by `asymmetric-channel`
and `server-http-api`.
`crates/unbill-device` docs are represented by `unbill-device`.
`crates/unbill-console` docs are represented by `unbill-console`,
`console-service`, `settlement`, `conflict-detection`, `ledger-doc`,
and `console-reexports`.
`crates/unbill-tauri` docs are represented by `unbill-tauri` and `unbill-ui-native`.
`crates/unbill-ui-components` docs are represented by `ui-components`.

Console submodule documentation:
`service` is represented by `console-service`.
`settlement` is represented by `settlement`.
`conflict` is represented by `conflict-detection`.
`doc` is represented by `ledger-doc`.
`storage` is represented by `unbill-storage`, `fs-store`, and `memory-store`.
`net` is represented by `console-reexports` and `symmetric-channel`.

Per-file audit:
`.github/DESIGN.md` maps to `ci-cd-pipeline`.
`.github/IMPLEMENTATION.md` maps to `ci-cd-pipeline`.
`AGENTS.md` maps to `development-workflow`.
`README.md` maps to `unbill`, `workspace-layout`, `distribution-and-release`,
and `ci-cd-pipeline`.
`DESIGN.md` maps to the root architecture entries listed above.
`IMPLEMENTATION.md` maps to `workspace-layout` and the crate entries listed above.
`apps/DESIGN.md` maps to `ui-shared-model`.
`apps/unbill-cli/DESIGN.md` and `apps/unbill-cli/IMPLEMENTATION.md` map to `unbill-cli`.
`apps/unbill-daemon/DESIGN.md` and `apps/unbill-daemon/IMPLEMENTATION.md` map to `unbill-daemon`.
`apps/unbill-server/DESIGN.md` and `apps/unbill-server/IMPLEMENTATION.md` map to
`unbill-server` and `server-http-api`.
`apps/unbill-tui/DESIGN.md` and `apps/unbill-tui/IMPLEMENTATION.md` map to `unbill-tui`.
`apps/unbill-ui-native/DESIGN.md` and `apps/unbill-ui-native/IMPLEMENTATION.md` map to
`unbill-ui-native`.
`apps/unbill-ui-remote/DESIGN.md` and `apps/unbill-ui-remote/IMPLEMENTATION.md` map to
`unbill-ui-remote`.
`crates/unbill-asymmetric-channel/DESIGN.md` and `IMPLEMENTATION.md` map to
`asymmetric-channel`.
`crates/unbill-console/DESIGN.md`, `IMPLEMENTATION.md`, and `README.md` map to
`unbill-console`, `console-service`, `settlement`, `conflict-detection`,
`ledger-doc`, and `console-reexports`.
`crates/unbill-console/src/conflict/DESIGN.md` and `IMPLEMENTATION.md` map to
`conflict-detection`.
`crates/unbill-console/src/doc/DESIGN.md` and `IMPLEMENTATION.md` map to `ledger-doc`.
`crates/unbill-console/src/net/DESIGN.md` and `IMPLEMENTATION.md` map to `console-reexports`.
`crates/unbill-console/src/service/DESIGN.md` and `IMPLEMENTATION.md` map to
`console-service`.
`crates/unbill-console/src/settlement/DESIGN.md` and `IMPLEMENTATION.md` map to
`settlement`.
`crates/unbill-console/src/storage/DESIGN.md` and `IMPLEMENTATION.md` map to
`unbill-storage`, `fs-store`, and `memory-store`.
`crates/unbill-device/DESIGN.md` and `IMPLEMENTATION.md` map to `unbill-device`.
`crates/unbill-event/DESIGN.md` and `IMPLEMENTATION.md` map to `unbill-event`.
`crates/unbill-model/DESIGN.md` and `IMPLEMENTATION.md` map to `unbill-model`.
`crates/unbill-storage/DESIGN.md` and `IMPLEMENTATION.md` map to `unbill-storage`
and `ledger-doc`.
`crates/unbill-store-fs/DESIGN.md` and `IMPLEMENTATION.md` map to `fs-store`.
`crates/unbill-store-memory/DESIGN.md` and `IMPLEMENTATION.md` map to `memory-store`.
`crates/unbill-symmetric-channel/DESIGN.md` and `IMPLEMENTATION.md` map to
`symmetric-channel`.
`crates/unbill-tauri/DESIGN.md` and `IMPLEMENTATION.md` map to `unbill-tauri`.
`crates/unbill-ui-components/DESIGN.md` and `IMPLEMENTATION.md` map to `ui-components`.

---

> **Sirno generated links begin. Do not edit this section.**

- belongs (to):
  - [development-workflow](development-workflow.md)
- belongs (from):
  - [visual-diagram-audit](visual-diagram-audit.md)

> **Sirno generated links end.**
