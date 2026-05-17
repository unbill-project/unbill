______________________________________________________________________

## name: sirno-editor description: >- Edit a Sirno Lake. Use when you create, revise, or reorganize compact Markdown entries, moves design knowledge from DESIGN.md or another mono form into the lake, chooses entry ids and structural metadata, expands entries, or validates generated Sirno links.

# Sirno Editor

## Purpose

Use this skill when editing `sirno` entries
or moving design knowledge from `mono` to `sirno`.
Editing keeps the lake precise, structured, and useful for future work.
Lowering gives a long-form narrative compact nominal form.
It preserves the design route while creating or revising entries
future work can cite, query, realize, and reflect.

Apply repository instructions first.
When the work also edits `README.md`, `DESIGN.md`, or `METHODOLOGY.md`,
use the repository documentation-writing skills for prose style and document roles.

## Core Judgment

Sirno Lake editing is not a heading split or metadata shuffle.
When lowering from a monograph,
read the monograph as a whole,
then name the durable objects that make future work easier to address.
When revising existing entries,
preserve stable ids and improve the structure that future work will cite.

Look for:

- concepts that compress repeated project knowledge
- categories that define project vocabulary
- entries that explain recommended structural fields and witness lookup
- transform entries that explain movement between forms
- interface, storage, metadata, and checking commitments
- witnessable claims that should connect to repository artifacts
- narrative routes that help a reader traverse concepts

Prefer entries that a future implementer or reviewer can point at.
Avoid entries that only restate a paragraph without creating a useful handle.

## Authority

Before editing, decide which form currently carries authority.
If the lake is already established and maintained,
treat it as the structured design source.
If the lake is absent or skeletal,
use the configured monograph as the source of intended design unless the user says code is authoritative.

Preserve stable facts from the current project model:

- central definition and scope
- configured `lake` path and optional `mono` path
- entry id rules and metadata schema
- configured structural field and witness lookup meanings
- generated footer ownership
- witness lookup conventions
- future-work items that are intentionally reserved

Translate stale language into current terms rather than preserving it literally.
Do not invent commands, guarantees, semantic understanding, or automatic validation that Sirno does not provide.

## Entry Design

Each entry should be small enough to read locally,
but substantial enough to stand on its own from a query result.
For foundational design entries, aim for reader-friendly prose of roughly a few paragraphs.
Shorter entries are fine when the concept is genuinely narrow.

Each entry should answer:

1. What does this name mean?
1. Why does it deserve a stable entry id?
1. How does it fit the project model?
1. Which structural metadata should carry its shape?
1. What should future implementation or review remember?

Use exact metadata for structure and prose for explanation.
Do not ask tools to infer structure from body text.

Use lowercase ASCII kebab-case ids.
Keep existing ids stable unless the user explicitly asks for a rename.
Use human-readable names and concise descriptions.

## Structural Field Model

This repository recommends the following structural field model.
Check `Sirno.toml` before relying on a field in commands or generated links.

Use `category` for kind.
An entry categorized by `concept` should define a compressed idea.
An entry categorized by `narrative` should record a route, story, or motivation through project ideas.
An entry categorized by `meta` should define the project's principles, vocabulary, or documentation method.
An entry categorized by `category` may itself be used as a category target.
The local documentation method requires that category targets are categorized by `category`.

Use `belongs` for review locality.
A `belongs` target is an entry that gives a shared subject or design region a front door.
It says that entries should be visited together because they live in the same working neighborhood.
The relation is horizontal.
It supports scanning, review, accountability, and local navigation across entries of different kinds.

Use `refines` for semantic narrowing.
A `refines` target is the broader entry that the current entry makes more specific.
It says that the current entry is a local, concrete, or testable version of another design claim.
The relation is vertical.
It preserves why an implementation detail, invariant, interface, route, or test belongs under a broader idea.

Prefer choosing either `belongs` or `refines` for a new entry.
They are suggested to be mutually exclusive because they answer different questions.
`belongs` answers "which review neighborhood contains this entry?"
`refines` answers "which broader claim does this entry specialize?"
Using both can blur locality and specificity,
so add both only when the entry truly sits in a review neighborhood
and also concretizes a broader design claim that should be followed separately.

When choosing `belongs`,
prefer the smallest set of targets that improves navigation, review, or accountability.
An entry may name several `belongs` targets only when each target is a real review perspective.
Keep split entries under the same `belongs` target when a small design change should be checked inside that unit.
Create a new `belongs` target only when there is a real new review boundary.

When choosing `refines`,
prefer the nearest broader entry that explains the current entry's design pressure.
Do not use `refines` to say that two entries are merely related or commonly edited together.
Create a more specific entry when a paragraph, code region, test, or policy needs a stable handle.

The entry id is the witness query key.
Discover evidence with `sirno witness ENTRY_ID --full`.
The body should briefly explain what the repository evidence is expected to demonstrate
when that meaning is not obvious from the entry claim.

When a structural field feels merely decorative,
leave it out.
Structural fields should improve navigation, review, or accountability.

## Prose Style

Write entries as durable design prose.
Define the positive rule first.
Use definition by negation only when it prevents a likely confusion.

Keep paragraphs focused.
One paragraph should introduce one idea, state its consequence, and stop.
Prefer short sentences and natural line breaks under the repository line budget.
Break Markdown prose at natural punctuation boundaries or conjunctions; otherwise don't make linebreaks.

Avoid turning the lake into:

- a glossary with labels but no design pressure
- a changelog that narrates edits instead of durable facts
- a task list that loses the concept behind the work
- a duplicate monograph split across files

Entries should be more local than the monograph,
but more durable than a plan item.

## Workflow

1. Read repository instructions, `Sirno.toml`, the configured monograph when present, and the existing lake.
   Use the configured lake as the routine edit target.
   `sirno-docs-zh/` stores the split Chinese translation snapshot.
   Leave that directory unchanged during lake maintenance and design sync.
1. Inspect the current Sirno CLI before assuming which commands exist.
1. Map candidate entries before editing:
   id, name, desc, structural fields, and witness status.
1. Create missing entries through Sirno's current entry-creation command when available.
1. Expand or revise bodies with direct, reader-friendly prose.
1. Leave generated footer regions untouched.
1. Run Sirno's generated-link command after metadata stabilizes.
1. Run structural checks and query commands to verify the lake parses and references resolve.

Use the configured lake path.
Do not hard-code `docs/` when `Sirno.toml` names a different lake.

## Document Search

Use `sirno query` to map concepts, structural neighborhoods, and candidate entry ids.
Read the `desc` field before deciding which entries to edit.

Use `sirno rg` to search literal text in Sirno documents:
phrases, command names, examples, stale wording, headings, or entry ids used in prose.
Plain `sirno rg` searches authored metadata and prose.
It ignores generated footer regions by default.
Use `sirno rg --with-generated-footer` only when generated links are the search target.

Useful document-search commands:

```text
sirno query TERMS --fields id,desc
sirno query --exact FIELD=ENTRY_ID --fields id,path,desc
sirno rg TEXT
sirno rg -n TEXT
sirno rg -C 2 TEXT
sirno rg --files
```

After finding literal matches,
read the matched entries before editing.
Do not rewrite from isolated match lines alone.

## Validation

Prefer these checks when the CLI provides them:

```text
sirno query --fields id
sirno rg TEXT
sirno check --mode edit
sirno gen-link
sirno check
sirno status
```

Use `cargo run -- ...` or `target/debug/sirno ...` according to the repository state.

If review-mode checks fail because local editor/tool directories are inside the lake,
preserve those files unless the user asks to remove them.
Report the blocker and still validate entry parsing and metadata references as far as possible.

## Git Hygiene

When asked to commit Sirno Lake editing work,
stage only the configured lake, the config change that points to it,
and directly related documentation.
Leave unrelated code or generated editor state alone.

Use the repository commit convention.
For documentation-only lake editing, `docs: revise sirno lake entries` is an appropriate shape.
