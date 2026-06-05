---
core.name: Refines
core.desc: A structural link relation from a specific entry to the broader entries it makes concrete.
meta.type: "structural"
meta.ripple.lake: ["to", "from"]
meta.ripple.anchor: ["from"]
core.category:
  - core.concept
core.belongs:
  - core.category
---

`refines` records a refinement edge from a more specific entry
to the broader entries it makes concrete.

Refinement turns broad design into more specific design,
implementation detail,
and testable behavior.
The current entry keeps the broader claim attached while making its consequences local.

Use `refines` when an entry answers the question:
what does this broader design mean here?
The field preserves the reason that a local choice exists.

The more specific entry points back to the broader entry.
This lets local work climb back toward intent,
and lets broad entries reveal the entries that elaborate them.

Use the nearest broader target that explains the current entry's design pressure.
Use `belongs` for entries that are merely reviewed together.
