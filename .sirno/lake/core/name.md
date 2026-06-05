---
core.name: Name
core.desc: The required plain-string title field for entries.
meta.type: "intrinsic"
core.category:
  - core.meta
  - core.concept
core.belongs:
  - core.category
---

`core.name` gives an entry its reader-facing title.

The value is a single-line plain string.
It should be short enough to scan in query output,
generated navigation,
and review surfaces.
The filename remains the entry address.
The `core.name` field is the human label for that address.

This entry exists so the core lake can define the standard title field for itself.
A project that imports the core lake can still keep its own intrinsic field registry.
