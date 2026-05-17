______________________________________________________________________

## name: readme-design-methodology-doc-writer description: >- Write or reorganize repository documentation split across README.md, DESIGN.md, and METHODOLOGY.md. Use when Codex needs to revise a first-impression README, a full design document, a concentrated methodology guide, or prose that defines a project model, metadata, structural fields, or writing style.

# README, Design, Methodology Doc Writer

## Purpose

Use this skill to write three public documentation forms as a coherent set:
`README.md`, `DESIGN.md`, and `METHODOLOGY.md`.

The documents should share one project model while serving different readers.
The README attracts and orients.
The design document explains the system.
The methodology document states the working principles.

## Document Roles

### README.md

Write `README.md` as the first impression.
Make it stylish, precise, and attractive to the project's intended audience
without making it promotional.
State the central idea early.
If the project has a logo, mascot, diagram, or primary visual,
place it near the top after the first paragraph unless the repository already
uses a stronger convention.

The README should intrigue readers enough to continue.
It should define the project compactly,
name the main forms or components,
and hint at the deeper design.
It should not carry the full design.

### DESIGN.md

Write `DESIGN.md` as the whole design document.
It should be readable in one sitting by a person who wants the details.

Give it fluent flow.
Introduce the project bit by bit.
Use concept-oriented sections whose transitions make the document feel like a
single narrative, not a pile of definitions.
Be willing to make it long when the design needs room.
Keep the prose declarative, dry, and precise.

Before and after editing `DESIGN.md`, evaluate it as a reader:

- Is the structure clear and logically ordered?
- Does the prose sound like a knowledgeable practitioner?
- Are redundant sections merged or removed?
- Does each paragraph do one thing and stop?

### METHODOLOGY.md

Write `METHODOLOGY.md` as a short, concentrated script.
It should state the principles and mindsets the project advocates.
It should be concrete enough to guide behavior,
but shorter and more forceful than `DESIGN.md`.
Do not use it as a second design document.

## Project Model

Extract the project model before rewriting.
Identify the project's canonical nouns, forms, transforms, structural fields,
metadata fields, and named operations from the current docs and user instructions.
Use those terms consistently across all three documents.

Prefer the user's latest instructions over older repository prose.
When existing documents disagree,
decide which statement is current before preserving it.
Translate useful old detail into the current vocabulary.

Do not invent commands, guarantees, or mechanisms that the project has not
claimed.
If a term is intended as methodology vocabulary,
do not describe it as an executable feature.

## Stable Facts

Preserve stable project facts unless the user explicitly changes them.
Typical stable facts include:

- the project's central definition
- the intended audience
- the main artifacts, stores, files, or runtime interfaces
- required metadata fields and their accepted shapes
- named structural fields and their semantics
- naming conventions
- generated regions and ownership boundaries
- external tools or libraries that provide specific mechanisms
- future-work items that should remain open

When examples are useful,
derive them from the project's current conventions.
Keep examples small enough to explain the rule they illustrate.

## Stale Or Risky Framing

Avoid these stale or misleading claims:

- Do not preserve obsolete terminology because it appears in an older file.
- Do not turn an optional workflow into a built-in primitive.
- Do not use broad structural words when the project has a specific field name.
- Do not imply semantic understanding where the project only provides a
  structural convention.
- Do not imply automatic validation, checking, or repair unless the project
  explicitly provides it.
- Do not make `README.md`, `DESIGN.md`, and `METHODOLOGY.md` redundant.

## Metadata And Structural Fields

When the project defines metadata, structural fields, or file conventions,
document their shape exactly.
State required fields, optional fields, accepted value forms, and ownership
rules.

Use examples that match the project's canonical syntax.
If the syntax is undecided, describe the design decision as open rather than
filling in a plausible shape.

When a document refers to generated content,
name the generated region and its ownership boundary.
If the generated region uses sentinels,
state that other tools and humans should leave that region untouched.

## Narrative Guidance

Use definition by affirmation.
Define what the project does before explaining what it avoids.
Use definition by negation only when motivation requires it.

Keep the writing declarative.
Prefer simple sentence structure.
Use technical terms once, then trust them.
Avoid rhetorical rebuttals, defensive caveats, and marketing cadence.

For `DESIGN.md`, connect sections with an elegant narrative line.
For `README.md`, let style carry curiosity without diluting precision.
For `METHODOLOGY.md`, compress into principles that can be acted on.

Break Markdown prose at natural punctuation boundaries or conjunctions.
Keep sentences short.
Avoid emojis.
Use bold text only when emphasis is truly useful.

## Rewrite Workflow

1. Read `README.md`, `DESIGN.md`, and `METHODOLOGY.md` before editing.
1. Check whether `README.md` is a symlink before editing.
   If its new role differs from `DESIGN.md`, replace the symlink deliberately.
1. Determine which document or user instruction is the current authority for
   each disputed term.
1. Preserve useful old details only after translating them into the current
   project model.
1. Keep the three documents distinct by role.
1. After editing, reread the changed documents for coherence, redundancy, and
   stale terminology.
