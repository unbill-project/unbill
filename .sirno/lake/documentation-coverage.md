---
core.name: Documentation Coverage
core.desc: Provenance record for the Sirno lake migration from per-crate Markdown.
core.category:
  - core.meta
core.belongs:
  - development-workflow
---

The pre-Sirno documentation lived in per-crate `DESIGN.md` and `IMPLEMENTATION.md` files,
plus root `DESIGN.md`, `IMPLEMENTATION.md`, `README.md`, and `AGENTS.md`.
All design facts from those files were reorganized into lake entries.
Each old file was checked before deletion.

Visual diagrams from the old documentation were recovered into the entries
that now own their claims.
Diagrams that needed correction were updated during the migration.

`README.md` was restored as a compiled artifact from `compiled-markdown-artifacts`.
The lake is now the authoritative design source;
root Markdown files are reader entry points compiled from it.
