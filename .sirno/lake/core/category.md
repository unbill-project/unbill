---
core.name: Category
core.desc: A structural link relation that classifies an entry by other entries.
meta.type: "structural"
meta.ripple.lake: []
meta.ripple.anchor: []
core.category:
  - core.category
  - core.meta
  - core.concept
---

`category` classifies an entry by other entries.

Categories are themselves entries.
This keeps project vocabulary open and documented instead of fixed by Sirno.

A category target should be categorized by `category`.
That marker says the target is usable as a kind.
The core lake uses this rule for `category`, `meta`, `concept`, and `narrative`.

Use `category` when the classified entry should be read as an instance of a named kind.
An entry categorized by `meta` defines the project's principles, vocabulary, or method.
An entry categorized by `concept` defines a compressed idea.
An entry categorized by `narrative` records or names a route through ideas.

Categories should stay semantic rather than decorative.
If a label only helps browsing by topic,
`belongs` may be a better fit.
If an entry makes another entry more concrete,
`refines` is the sharper field.
