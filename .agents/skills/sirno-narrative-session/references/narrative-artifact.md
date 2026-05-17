# Narrative Artifact Reference

## Session Notes

Keep notes compact.
They are scaffolding for the final route,
not a transcript.

Recommended fields:

```json
{
  "reader": "new contributor",
  "task": "understand how Sirno Lake entries guide implementation",
  "pressure": "the reader needs a route before editing code",
  "pull": "why a stable entry id matters before the edit begins",
  "known_terms": ["entry", "lake"],
  "missing_terms": ["belongs", "refines", "witness"],
  "route": [
    {
      "entry": "introduction",
      "role": "first route through the project model",
      "prerequisite": "project shape",
      "deferred": "detailed metadata rules"
    }
  ],
  "feedback": [
    "Move witnesses later because the reader has not seen metadata yet."
  ],
  "aftertaste": "name the thing before the work becomes local",
  "artifact": {
    "id": "new-contributor-route",
    "name": "New Contributor Route",
    "desc": "A narrative route for a new contributor learning Sirno."
  }
}
```

## Entry Metadata

Use this shape for a materialized narrative entry:

```yaml
---
name: New Contributor Route
desc: A narrative route for a new contributor learning Sirno.
category:
  - narrative
belongs:
  - sirno
refines:
  - narrative
---
```

Omit empty fields.
Add `witness:` only when repository evidence exists.

## Entry Body Shape

Use prose paragraphs by default.
Use bullets only when they make the route easier to scan.

The body should answer:

1. Who is this route for?
1. What design pressure makes the route useful?
1. What pull or tension makes the next concept worth meeting?
1. What should be understood first?
1. What entries carry the ordered route?
1. What local detail is deferred?
1. What phrase, handle, or entry id should remain afterward?
1. What durable feedback shaped the route?

Avoid copying whole entry definitions into the route.
Name the entries and explain why they appear in that order.

## Serializer Input

`scripts/serialize_narrative_entry.py` accepts JSON with these fields:

```json
{
  "id": "new-contributor-route",
  "name": "New Contributor Route",
  "desc": "A narrative route for a new contributor learning Sirno.",
  "structural": {
    "category": ["narrative"],
    "belongs": ["sirno"],
    "refines": ["narrative"]
  },
  "body": [
    "This route serves a new contributor who needs the project model before editing code.",
    "The pull is simple: a stable entry id lets the work name its design object early.",
    "Start with `introduction`, then read `methodology`, then visit `entry` and `narrative`.",
    "Detailed metadata rules stay in their own entries until the route needs them.",
    "The aftertaste is `name the thing before the work becomes local`."
  ]
}
```

The script writes structural fields exactly as supplied in `structural`.
It refuses to overwrite an existing entry unless `--force` is passed.
