---
name: Compiled Markdown Artifacts
desc: Root Markdown files regenerated from the Sirno lake for humans and agents.
category:
  - meta
belongs:
  - development-workflow
---

`README.md` and `AGENTS.md` are repository-facing artifacts compiled from this lake.
They are useful entry points,
but they are not the design source.

Durable design and workflow changes start in `unbill-docs`.
After the lake is updated and checked,
root artifacts can be regenerated so external readers and coding agents have a short route in.

`README.md` compiles product identity, repository shape, build and release routes,
and pointers into the lake.
Its sources are `unbill`, `workspace-layout`, `distribution-and-release`,
`ci-cd-pipeline`, and `introduction`.

`AGENTS.md` compiles the development workflow.
Its source is `development-workflow`,
with `documentation-coverage` and `visual-diagram-audit` as audit support.

The root artifacts should stay concise.
They should point to lake entries instead of duplicating every local crate or module fact.

---

> **Sirno generated links begin. Do not edit this section.**

- belongs (to):
  - [development-workflow](development-workflow.md)
- belongs (from): (none)

> **Sirno generated links end.**
