______________________________________________________________________

## name: sirno-explorer description: >- Explore a Sirno-managed repository by using `sirno query`, exact structural predicates, `sirno witness`, and targeted code inspection. Use when you need to understand a codebase through its Sirno Lake, find relevant entries before editing, trace design concepts to repository evidence, map an implementation area, answer questions about where behavior lives, or prepare a change by following Sirno entries and witness blocks.

# Sirno Explorer

## Purpose

Use this skill to explore a Sirno-managed repository from the lake outward.
The Sirno Lake provides the project map.
Witness blocks provide repository evidence for entry claims.

Exploration should produce a small, grounded route:
entry ids, why they matter, witness locations, and the code or docs they point to.

## Core Workflow

1. Locate the active project config.
   Prefer `Sirno.toml` in the current repository.
   Read `[lake].path`, `[structural]`, and `[repo].members` before assuming paths.

1. Start with Sirno query.
   Use vague query for discovery:

```sh
cargo run -- query TERMS --fields id,desc
```

Use exact structural predicates when the route is known:

```sh
cargo run -- query --exact FIELD=ENTRY_ID --fields id,desc
cargo run -- query --exact FIELD=ENTRY_ID --exact OTHER_FIELD=OTHER_ENTRY_ID --fields id,desc
```

Read the `desc` field before narrowing the route.
It gives each candidate's intended meaning and prevents id-only matching.

3. Read the most relevant entry files from the configured lake.
   Prefer a few high-signal entries over broad scans.
   Follow `belongs`, `refines`, and configured structural fields when they clarify the route.
   Ignore generated footer links as authority;
   they project metadata and may be useful only as navigation hints.

1. Ask Sirno for repository evidence.
   For each likely entry id, run:

```sh
cargo run -- witness ENTRY_ID --full
```

If no witness exists, say that directly.
Then inspect the entry prose and related entries before using literal text search.

5. Inspect witnessed repository regions first.
   Read the files and nearby context around witness spans.
   Use `sirno rg` for focused follow-up searches inside the lake.
   Use plain `rg` when searching repository code outside the lake.
   Keep the entry claim and the code evidence connected in your notes.

1. Synthesize the route.
   Report what you found as:
   relevant entries, witness locations, code/doc locations, and remaining uncertainty.
   Prefer file and line references over broad summaries.

## Lake Discovery

Use `sirno query` when the user's language is conceptual,
when structural metadata should guide the route,
or when you need entry descriptions.

Start vague for discovery:

```sh
cargo run -- query parser metadata --fields id,desc
```

Start exact when the user names a structural field or known entry id:

```sh
cargo run -- query --exact FIELD=ENTRY_ID --fields id,desc
```

Combine vague and exact filters when useful:

```sh
cargo run -- query generated footer --exact FIELD=ENTRY_ID --fields id,desc
```

Use `--fields id,path,desc` when you need entry file paths from the result set.
Use the configured structural field names from `Sirno.toml`.
This repository recommends `category`, `belongs`, and `refines`,
but commands should use the structural fields configured in `Sirno.toml`.

Use `sirno rg` when you need literal text inside Sirno documents:
phrases, command names, examples, old wording, headings, or entry ids used in prose.

```sh
cargo run -- rg generated-footer
cargo run -- rg -n "generated footer"
cargo run -- rg -C 2 "with-generated-footer"
cargo run -- rg --with-generated-footer generated-footer
cargo run -- rg --files
```

`sirno rg` forwards arguments to the real `rg` command and appends the configured lake path.
It ignores generated footer regions by default.
Use `--with-generated-footer` when generated links are the search target.
Use plain `rg` only for repository code or files outside the configured lake.

After a literal match,
read the matched entry body and its metadata before treating the line as design authority.

## Witness Strategy

Treat witness output as evidence, not as a replacement for reading code.
A witness region says where to inspect a claim.
It does not prove that the code is correct.

When witness output is broad:

- read the whole region once
- identify the smallest relevant function, test, or config stanza
- use local search for nearby callers, tests, and types
- mention if the witness would benefit from splitting

When an entry has no witness:

- check related entries through configured structural fields
- search the repository for the entry id and key terms
- state whether the result is documentation-only, unwitnessed, or not found

## Exploration Discipline

Keep the route narrow.
Avoid reading the whole lake or whole repository unless the question truly asks for a survey.

Prefer this order:

1. `sirno query`
1. entry metadata and prose
1. `sirno witness ENTRY_ID --full`
1. witnessed files and nearby code
1. targeted `sirno rg` or plain `rg`

Do not add or edit witness blocks while exploring.
Use `sirno-witness` when the task changes from exploration to creating or refining evidence.
Use `sirno-editor` when the task changes from exploration to editing lake entries.

## Reporting

Answer with grounded findings.
Name the route taken when it helps the user trust the result.

Good exploration output includes:

- entry ids consulted
- descriptions that shaped the route
- witness files and line ranges
- code symbols or docs inspected
- what is known, inferred, and still uncertain
- suggested next inspection step when useful

If checks fail, report the blocker and continue with evidence that can still be inspected safely.
