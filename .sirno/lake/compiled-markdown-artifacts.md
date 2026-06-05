---
core.name: Compiled Markdown Artifacts
core.desc: Root Markdown files regenerated from the Sirno lake for humans and agents.
core.category:
  - core.meta
core.belongs:
  - development-workflow
---

`README.md` is a repository-facing artifact compiled from this lake.
It is a useful entry point,
but it is not the design source.

`INSTALL.md` is the per-platform installation guide.
It covers macOS, Linux, Windows, iOS, Android, and Docker,
listing every available method per platform.
Its source is `distribution-and-release` and `altstore-source`.

Durable design and workflow changes start in `unbill-docs`.
After the lake is updated and checked,
root artifacts can be regenerated so external readers have a short route in.

`README.md` compiles product identity, repository shape, build and release routes,
and pointers into the lake.
Its sources are `unbill`, `workspace-layout`, `distribution-and-release`,
`ci-cd-pipeline`, and `introduction`.

The root artifact should stay concise.
It should point to lake entries instead of duplicating every local crate or module fact.
