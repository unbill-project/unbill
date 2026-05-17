______________________________________________________________________

## name: sirno-witness description: >- Create, refine, or review Sirno repository witnesses. Use when you insert or split `sirno:witness:<entry-id>:begin` blocks, links code/tests/config to Sirno Lake entries, decides whether a witness needs a new or more specific Sirno Lake entry, interprets `sirno witness` output, or checks whether a Sirno entry has precise repository evidence.

# Sirno Witness

## Purpose

Use this skill when linking Sirno Lake entries to repository evidence.
A witness is a repository region that makes an entry claim inspectable.
The entry id is the query key.

## Core Principles

Keep the lake entry and repository artifact separate.
The entry states the claim in project language.
The witness block identifies where that claim can be inspected in code, tests, config, assets,
or generated artifacts.

Use repository witness blocks only when repository evidence exists.
Do not describe future evidence as present.
Do not invent witness ids or query strings separate from the entry id.

Prefer precise witness regions.
A block should cover the smallest durable region that supports the entry claim.
Multiple small blocks for one entry are better than one broad block that forces a reviewer to hunt.

Create a new entry when the evidence supports a related but different claim.
Do not reuse a near-enough entry id just to avoid lake editing.
Create a more specific entry when the evidence supports a narrower claim.
Use the configured structural fields for navigation.
This repository recommends `refines` for broader-entry targets and `belongs` for review locality.
Keep the same `belongs` target when the evidence belongs to the same module-like review unit.
Add another `belongs` target when the entry sits at a real intersection that should be reviewed from both sides.
Create a new `belongs` target only when the evidence belongs to a new design/program boundary.
Avoid ad hoc suffixes such as `entry#parser`; use real entry ids.

Do not duplicate `mosaika` behavior in Sirno.
Let Sirno call `mosaika` for delimiter matching, region extraction, and spans.
Sirno-side code should consume structured scan output and format it for review.

## Linking Workflow

Read the target entry before editing code.
Understand the claim, its structural fields, and any body guidance about what evidence should mean.

If no existing entry matches the witness need precisely,
create or propose a compact Sirno Lake entry before adding markers.
Keep the witness id tied to that exact entry claim.

Inspect current witnesses before adding new ones:

```sh
cargo run -- witness ENTRY_ID --full
```

Choose the evidence region deliberately.
Prefer a single item, test case, config stanza, generated boundary, or small cohesive block.
If the current region is too broad, split it into smaller blocks with the same entry id.

Add the repository block inside configured `[repo].members` paths.
If no repo members are configured, configure the intended repository artifact surface first:

```rust
// sirno:witness:entry-id:begin
// witnessed repository region
// sirno:witness:entry-id:end
```

For Markdown artifacts, use hidden HTML comment sentinels instead of visible line comments.
Always check `[[witness.delimiters]]` before adding markers.
Use the configured delimiter syntax.

For Rust, place witness comments around stable items or focused implementation spans.
Avoid wrapping unrelated helpers just because they are nearby.

Update the entry prose when needed so it briefly says what the witness region demonstrates.
Leave generated footer regions untouched.

## Validation

After adding or changing witnesses, run the direct witness query:

```sh
cargo run -- witness ENTRY_ID --full
```

Then run structural validation:

```sh
cargo run -- check --mode review
```

If Sirno Lake metadata or links changed, run:

```sh
cargo run -- gen-link
```

Review the full witness output as a human would.
The output should show concise ranges, literal matched regions, and no broad unrelated code.
