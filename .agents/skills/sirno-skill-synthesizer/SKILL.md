______________________________________________________________________

## name: sirno-skill-synthesizer description: >- Rebuild the packaged Sirno agent skills from the lake. Use when you regenerate or audit `.agents/skills/sirno-*/SKILL.md`, after editing a `meta`-categorized discipline entry, when a packaged skill drifts from the Sirno Lake, or when `agent-skills` adds a new skill to the roster.

# Sirno Skill Synthesizer

## Purpose

Use this skill to rebuild the packaged agent skills from the Sirno Lake.
The lake's `meta`-categorized entries are the source of truth.
A packaged `SKILL.md` is their reproducible surface, not a separate authority.

The procedure this skill encodes lives in the `skill-synthesis-discipline` entry.
This skill is itself rebuilt from that entry the same way it rebuilds the others.

## Core Principles

Read the lake before writing any skill.
Read `Sirno.toml` for the lake path,
`agent-skills` for the skill roster and the handoffs between skills,
and every `meta`-categorized entry for the method the skills render.

Separate disciplines from shared method.
A `meta` entry that `belongs: agent-skills` and states a skill procedure is a skill source.
The other `meta` entries carry vocabulary, principles, perspective, and design authority.
They are cross-cutting method every skill must respect, not skills in their own right.

Map each discipline to exactly one package.
A skill discipline renders one `.agents/skills/sirno-<role>/SKILL.md`.
Keep the existing skill directory name.
Do not invent a new role unless `agent-skills` adds one to the roster.
Every `belongs: agent-skills` discipline should have a package,
and every package should trace back to a discipline.

Render, do not reinterpret.
A packaged skill operationalizes its discipline plus the shared `meta` method it depends on.
Add nothing the lake does not commit, and drop nothing the discipline requires.
When a packaged skill and the lake disagree, correct the skill, never the lake.

## Synthesis Workflow

Read the roster and the method.

```sh
cargo run -- query --exact category=meta --fields id,desc
cargo run -- query --exact belongs=agent-skills --fields id,path,desc
```

Read `agent-skills` and each discipline entry in full before rendering.
Read a matched entry rather than working from the `desc` line alone.

Classify the `meta` entries.
The `belongs: agent-skills` discipline entries are skill sources, one package each.
The remaining `meta` entries are shared method.
Fold the shared method into the skills that depend on it,
for example design authority and the structural-field model into the editor skill.

Render each package.
Write `.agents/skills/sirno-<role>/SKILL.md` with valid frontmatter:

```yaml
---
name: sirno-<role>
description: >-
  When to use the skill and the triggers that should invoke it.
---
```

Then write the body as direct procedure:
purpose, core principles, an ordered workflow, and validation.
Turn durable procedure into concrete steps and current commands.

Inspect the current Sirno CLI before writing commands into a skill.

```sh
cargo run -- --help
```

A skill that names a missing command is worse than one that only names the procedure.
Use `cargo run -- ...` or `target/debug/sirno ...` according to the repository state.

## Validation

If lake metadata or links changed, run generated-link maintenance:

```sh
cargo run -- gen-link
```

Then run the review-mode structural check:

```sh
cargo run -- check --mode review
```

Confirm each `SKILL.md` has valid frontmatter.
Confirm the disciplines and packages still correspond one to one:
no `belongs: agent-skills` discipline without a package,
and no `sirno-*` package without a discipline.

Report the skills rebuilt, the entries they were rendered from,
and any discipline or package that no longer has a counterpart.
