______________________________________________________________________

## name: design-doc-writer description: >- Write and revise a repository's DESIGN.md or equivalent design document. Use when Codex edits a design document, evaluates its structure, reorganizes design prose, removes overlap, or applies a declarative, dry, precise design-document style.

# Design Doc Writer

## Purpose

Use this skill when editing `DESIGN.md` or an equivalent project design document.
A design document explains the system through ordered concepts and exact terms.
It should be readable in one sitting by a reader who wants the details.

Apply the target repository's instructions first.
Use this skill for structure and prose when the repository has no stronger local rule.

## Reader Evaluation

Before and after any edit, evaluate the document as a reader:

- Is the structure clear and logically ordered?
- Does the prose read like it was written by a knowledgeable practitioner?
- Are there redundant or overlapping sections that should be merged or reordered?
- Does each paragraph introduce one concept, state its properties, and stop?

Apply these standards to every edit.

## Prose Style

Write in a declarative, dry, precise style.
Prefer short main clauses over nested subordination.
Use an impersonal voice without becoming bureaucratic.
Write closer to concise mathematical prose than to a software README.

Do not use motivational framing, rhetoric, or "this is important because."
Introduce terms once, then trust them to carry the rest of the text.
Use analogies sparingly and only to established PL concepts,
such as well-typedness or nominal binding.
Do not use everyday metaphors.

## Structure

Order sections by conceptual dependency and scope.
Define a term before using it to state a rule.
Merge sections that describe the same concept at different levels of detail.
Move local details near the concept they constrain.

Prefer direct definitions over defensive framing.
State the positive rule first when documenting a constraint.
Use definition by negation only when a nearby confusion is likely.

## Workflow

1. Read the whole target document before editing.
1. Identify the concepts, terms, and section order that the edit touches.
1. Apply the smallest structural change that makes the document clearer.
1. Rewrite prose to match the style rules above.
1. Reread the changed section and its neighboring sections for flow and overlap.
1. Reread the whole document when the edit changes section order or terminology.
