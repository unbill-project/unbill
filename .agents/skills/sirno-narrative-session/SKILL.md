______________________________________________________________________

## name: sirno-narrative-session description: >- Conduct adaptive Sirno narrative sessions with users and materialize the resulting route as a Sirno Lake entry. Use when you need to teach, onboard, review, or explore project knowledge through questions, feedback loops, reader-state tracking, narrative route design, or serialized narrative artifacts in `sirno-docs/`.

# Sirno Narrative Session

## Purpose

Create an interactive route through Sirno knowledge for a particular reader or task.
The session may be ephemeral while the user is learning,
but the final artifact should be a compact Sirno narrative entry when the user asked for one.

Treat entries as the durable source of knowledge.
The narrative chooses sequence, prerequisites, pressure, and deferral.
It should not duplicate the whole lake.

## Reader Pull

Make knowledge feel worth moving toward before making it complete.
The pull may be practical, aesthetic, playful, urgent, elegant, sexy, relieving, or clarifying.
Desire is safe to name when it is the route's real pull.
Do not reduce every route to appetite, sexiness, or any single desire.

Use these heuristics while designing the route:

- Pull before explanation: show the tension before giving the name.
- Clean first bite: give the smallest useful version before the full model.
- Texture: mix definition, example, contrast, consequence, and a good name.
- Sequence: reveal the next useful part, then let one idea unlock the next.
- Agency: ask what the reader is trying to do, then route knowledge toward that action.
- Aftertaste: leave a phrase, handle, or entry id the reader can reuse later.

Do not force every move into every answer.
Use the moves that make the next concept arrive at the right time.

## Source Reading

Before designing the route, read:

- `Sirno.toml` for the configured lake path
- `sirno-docs/narrative.md`
- `sirno-docs/introduction.md`
- `sirno-docs/methodology.md`
- any entries named by the user or implied by the task

Read `references/narrative-artifact.md` when preparing the session notes or serialized entry.
Use `scripts/serialize_narrative_entry.py` when a deterministic entry draft is useful.

## Session Workflow

Start by naming the session frame in one or two sentences.
State the likely route goal and the current uncertainty.

Ask targeted questions when the answer would change the route.
Prefer one question at a time.
Use at most three questions in a single turn.
Good questions identify the reader's task, prior vocabulary, desired depth, confusion point,
or preferred artifact shape.

Maintain a small private session state:

- reader and task
- design pressure that makes the route useful
- pull or tension that makes the next concept worth meeting
- known terms and missing prerequisites
- ordered entry route
- user feedback and corrections
- deferred details
- aftertaste phrase, handle, or entry id
- intended narrative entry id, if materializing

Loop in short segments:

1. Explain the next concept or route choice.
1. Ask for feedback, confirmation, or a concrete input when it affects the next step.
1. Revise the route when the user shows confusion, boredom, urgency, or a sharper goal.
1. Name what moved earlier, what moved later, and why.

Prefer questions that unlock better sequencing.
Do not ask questions only to create a feeling of interactivity.
When the user wants momentum and the next step is clear,
continue and state the assumption.

## Route Design

A good route makes accurate concepts arrive at the right time.
Choose what must be understood first,
what can be named and deferred,
and what local detail should stay in entries.

For each route step, record:

- entry id or proposed entry id
- role in the route
- prerequisite it satisfies
- detail deferred to the entry body or another entry

Use existing entry ids exactly.
Create proposed ids only when the session discovers a missing durable object.

## Materializing The Narrative

Materialize a narrative entry when the user requests a saved route,
when the route will guide future onboarding or review,
or when the session produces a reusable way through a design region.

Choose a lowercase kebab-case id.
Use configured structural metadata.
This repository recommends `category: narrative` for narrative entries,
`belongs` for the project area the route belongs to,
and `refines: narrative` when the entry is a specific form of the general narrative concept.
Add other `refines` targets only when the route makes a broader entry concrete.

The entry body should state:

- who the route serves
- why the route matters
- what pull or tension made the route useful
- useful prerequisites
- the ordered route through entries
- what detail is intentionally deferred
- user feedback that changed the route, if durable
- the phrase, handle, or entry id the reader should carry forward

Keep the body short enough to read in place.
Point to entries that carry durable detail instead of copying their contents.
Do not include private chat transcript unless the user explicitly asks.

Use the serializer from the skill directory when helpful:

```sh
python3 .agents/skills/sirno-narrative-session/scripts/serialize_narrative_entry.py \
  --lake sirno-docs \
  --input session.json
```

After changing lake metadata, run generated-link maintenance.
Then run structural checks.
Prefer the repository's current Sirno CLI commands after inspecting what exists,
such as:

```sh
cargo run -- gen-link
cargo run -- check --mode edit
```

## Handoff

Finish by naming the route artifact, the entry path, and the main sequencing choice.
Mention any assumption that shaped the route.
Check that the route preserves pull, a clean first bite, and an aftertaste.
If checks were not run, say why.
