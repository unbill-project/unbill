---
core.name: Belongs
core.desc: A structural link relation that places an entry in a review neighborhood.
meta.type: "structural"
meta.ripple.lake: ["to", "from", "clique"]
meta.ripple.anchor: ["from"]
core.category:
  - core.concept
core.belongs:
  - core.category
---

`belongs` places an entry in a named review neighborhood.

The target entry gives a shared subject,
local vocabulary,
or design region a front door.
The relation is horizontal.
A local design or program change should often be reviewed by visiting that target,
its members,
and their related entries or evidence.

Use `belongs` when entries should be visited together because they share working context.
The field does not say that the member is an instance of a kind
or that it makes the target entry more concrete.
Use `category` for kind.
Use `refines` when the current entry narrows a broader design claim.

Keep `belongs` targets sparse.
A target should help navigation,
review,
or accountability.
