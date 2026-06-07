---
core.desc: The structure and tooling for formally verifying unbill's ledger invariants with Verus.
core.name: Formal Verification
core.category:
  - core.meta
core.belongs:
  - unbill
core.refines:
  - development-workflow
---

Unbill uses Verus for unbounded deductive verification of its core ledger algorithms.
Verus proves `requires`/`ensures` contracts via Z3, directly on Rust code inside `verus!{}` blocks.

The properties to verify are defined in `formal-invariants`.

## Toolchain

Verus is a modified Rust compiler. It ships as a prebuilt binary with bundled Z3.
The devenv packages it via a Nix derivation wrapping the prebuilt release.
The wrapper shims `rustup` so Verus finds its bundled Rust 1.95.0 toolchain.

All verified crates depend on `vstd` from crates.io.
Under `cargo build`, `verus!{}` expands to valid Rust with ghost code erased.
Under `verus`, the full verification runs.
Verified crates use the workspace edition (2024).

## Three-layer model architecture

The verification model mirrors the production architecture in three layers,
from bottom to top:

### Layer 1: LedgerStateModel

The lowest layer. Structurally equivalent to the production `Ledger` struct.
Contains ledger_id, users, bills, devices — every field the production type has.

Spec types use `Seq` for reasoning (`spec.rs`).
Runtime types use `Vec` for execution (`exec.rs`).
Two conversion functions bridge the production CRDT `Ledger` and the model:
- `ledger_to_model`: production Ledger → LedgerStateModel
- `model_to_ledger`: LedgerStateModel → production Ledger
Their correctness is covered by tests, not Verus,
because Automerge types are outside Verus's reach.

Each ledger operation (add_user, add_device, add_bill) is a single transition predicate
`op(pre, post, input) -> bool` that defines a valid move.
A state machine invariant is proved preserved by each transition.

### Layer 2: DeviceStateModel

Represents one device's local state. A struct containing:
- `device_id`: the device's own identity.
- `ledgers`: the ledgers this device manages.
- Future: local metadata, sync state.

This matches the production architecture where each device
(CLI, TUI, Tauri app, daemon) holds multiple ledgers,
each backed by a separate Automerge document.

### Layer 3: WorldModel

The global picture. Contains:
- `devices`: the set of all devices in the system.
- `ulid_state`: tracks all generated IDs to ensure global uniqueness.
- Future: network model for sync/merge.

ULID uniqueness is modeled as a formal property:
`ulid_unique(state)` asserts all generated IDs are distinct.
We *trust* that `Ulid::new()` produces fresh IDs (axiom),
but we *describe* the uniqueness property formally
so downstream proofs can depend on it.

The three layers are designed so that future modules can:
- Prove conservation across bill operations (layer 1).
- Prove device-level invariants (layer 2).
- Prove sync correctness and global uniqueness (layer 3).

## Code structure

Each verified module has up to four files:

```
spec.rs     -- spec types (Seq-based) and interface predicates
proof.rs    -- proof fn lemmas for invariant preservation
exec.rs     -- runtime types (Vec-based) and exec functions
mod.rs      -- module re-exports
```

`spec.rs` is kept short: only spec fn used in contracts.
`proof.rs` contains proof utilities referenced only from `proof {}` blocks.
`exec.rs` contains runtime types and exec functions callable by production code.

## Why separate verified crates

Verus cannot process crates that use `dyn` trait objects.
`thiserror` and `autosurgeon` generate `dyn Error`,
so production model crates cannot be compiled by Verus.

Verified crates isolate pure logic from the `dyn`-heavy dependency graph.
IDs are modeled as `Seq<u8>` (spec) / `Vec<u8>` (exec),
matching the production ULID string representation.
Production code maps domain types to/from verified types at the boundary.

## Contract pattern

Each public exec function has a single requires and ensures predicate,
named after the function:

```
requires spec::split_shares_requires(shares@, total_cents),
ensures  spec::split_shares_ensures(shares@, total_cents, result@),
```

Each ledger operation is a single spec predicate `op(pre, post, input) -> bool`.
The state machine invariant is separate — not embedded in the transition predicate.
Preservation is proved as: `invariant(pre) && op(pre, post, input) ==> invariant(post)`.

## Clippy

Verified crates suppress clippy lints at the crate level:
`ptr_arg` (Verus requires `&Vec` not `&[_]`),
`assign_op_pattern` (Verus requires `a = a + b` not `a += b`),
`len_zero` (Verus requires `len() == 0` not `is_empty()`),
`deprecated` (vstd's `SliceAdditionalExecFns::set`),
`unused_imports` (vstd imports erased under cargo).

## Verification workflow

1. Define spec types and predicates in `spec.rs`.
2. Write proof lemmas in `proof.rs`.
3. Implement exec functions in `exec.rs` with contracts and proof blocks.
4. Run `verus --crate-type lib <crate>/src/lib.rs`.
5. All verified code compiles with `cargo build` (`verus!{}` strips ghost code).
6. Write tests for the spec↔production type conversion functions.

## What works well for verification

Mathematical properties with clear invariants:
- Sum preservation (split_shares conservation).
- Fairness bounds (each share within 1 cent of ideal).
- Overflow absence (arithmetic stays within type bounds).

These are properties that tests can't fully cover
because the input space is too large for exhaustive testing.

## What is less suited for verification

Structural validation (IDs unique, references valid)
that production code already enforces at runtime.
The proof effort is high and the insight is low —
the invariants restate checks that `ops.rs` already performs.

The Vec↔Seq bridge (mapping between runtime Vec and spec Seq)
is mechanical and tedious. Minimizing the gap between
spec types and exec types reduces this overhead.

## Lessons learned

### Verus requires explicit loop structure
Iterator chains (`iter().map().sum()`) have no contracts in vstd.
Loops must be explicit `while` with `invariant` and `decreases` clauses.

### `nonlinear_arith` is essential for multiplication/division
Z3's default solver cannot reason about products and quotients.
Use `assert(...) by(nonlinear_arith) requires ...;` to dispatch
nonlinear goals to a specialized solver.

### `ext_eq` (`=~=`) for sequence equalities
When proving two `Seq` values are equal (e.g., `s.push(x).drop_last() =~= s`),
use extensional equality. The solver cannot derive this from axioms alone.

### Modular arithmetic needs manual case analysis
Z3 struggles with `%`. Use vstd `lemma_fundamental_div_mod` to decompose,
`lemma_fundamental_div_mod_converse_mod` to establish remainders,
and case-split on whether the sum wraps around.

### Sum-of-floors requires a proportional bound
Prove `floor_sum * W <= t * Σw_i` by induction.
Each step uses `floor_last * W <= t * w_last`.
When `Σw_i == W`, conclude `floor_sum <= t`.

### Ghost state for remainder distribution
Track `floor_amounts` snapshot and per-element invariant (floor or floor+1).
Use `mod_distinct` lemma to prove each index visited at most once.

### Overflow requires preconditions
Verus checks every arithmetic operation for overflow.
Add bounded preconditions and track bounds through loop invariants.

### Vec↔Seq bridge lemmas
To connect runtime `Vec<T>` views to spec `Seq<T::View>`:
- Use `assert(vec@.map(|i, x| x@)[k] == vec@[k]@)` to distribute map.
- Prove `has_foo_true` and `has_foo_false` lemmas by contradiction or witness.
- Triggers cannot contain lambdas — trigger on `vec[k]` not `vec.map(f)[k]`.
