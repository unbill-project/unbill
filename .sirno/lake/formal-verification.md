---
core.desc: The structure and tooling for formally verifying unbill's ledger invariants with Verus.
core.name: Formal Verification
meta:
  frozen:
    - reviewed
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
Under `cargo-verus`, the full verification runs.
Verified crates use the workspace edition (2024).

### cargo-verus for cross-crate verification

The standalone `verus` command cannot resolve Cargo dependencies.
When one verified crate depends on another (e.g. console-verified → model-verified),
use `cargo-verus verus verify -p <package>` instead.

cargo-verus uses `RUSTC_WRAPPER` to invoke the Verus compiler through Cargo.
It needs a `rustup` shim on PATH because Verus internally calls `rustup run`.
The devenv and CI both provide this shim.

Each verified crate must declare `[package.metadata.verus] verify = true`
in its `Cargo.toml` for cargo-verus to process it.

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

## Production wiring

All mutations in `ops.rs` flow through verified try_* wrappers:

```
CRDT → hydrate → Ledger → ledger_to_model → LedgerState
                                                 ↓ (check + exec)
CRDT ← reconcile ← Ledger ← model_to_ledger ← LedgerState
```

Each operation has three layers in the verified crate:
- **check function** (`check.rs`): runtime validation with user-friendly errors,
  proven to imply the exec requires predicate when it returns Ok.
- **exec function** (`exec.rs`): the actual mutation, proved to preserve `ledger_invariant`.
- **try wrapper** (`exec.rs`): composes check + exec, returns `Result`.

`ops.rs` calls `try_add_user`, `try_add_device`, `try_add_bill`.
No ad-hoc validation in ops.rs — the checker handles all precondition checks.
The only `requires` on the try wrappers is `ledger_invariant`,
which is maintained inductively (init establishes it, every mutation preserves it).

Per-operation error types (`AddUserOpError`, `AddDeviceOpError`, `AddBillOpError`)
in `error.rs` combine the verified check error with a reconcile error variant.
These convert to `UnbillError` via `#[from]` for callers that use the catch-all.

The bridge (`verified_bridge.rs` in `unbill-model`) provides:
- `ledger_to_model(ledger: &Ledger) -> LedgerState`
- `model_to_ledger(model: &LedgerState) -> Result<Ledger, BridgeError>`

Round-trip tested: `ledger_to_model ∘ model_to_ledger` preserves all fields.

The key bridge proof lemma `seq_map_push` connects `Vec::push`
through `View` to spec `Seq::push`, enabling Verus to verify
that the exec push satisfies the spec transition predicate.

## Code structure

Each verified module has up to four files.
Not all are required — omit files that would be empty.

```
spec.rs     -- spec types (Seq-based) and interface predicates
proof.rs    -- proof fn lemmas invoked from `proof {}` blocks
exec.rs     -- runtime types (Vec-based) and exec functions
mod.rs      -- module root; re-exports and/or exec code
```

### spec.rs

The public interface of the module's verification.
Contains only `open spec fn` definitions used in `requires`/`ensures` contracts:
precondition predicates, postcondition predicates, type definitions, helper specs.
No proof logic. No exec code. Kept minimal.

### proof.rs

Proof-only utilities: `proof fn` lemmas and helper `open spec fn` definitions
that exist only to guide Z3. These are invoked from `proof {}` blocks
inside exec functions. Never called at runtime.

### exec.rs

Runtime types and exec functions that production code can call.
Each exec function has contracts (`requires`/`ensures`) referencing spec.rs,
and `proof {}` blocks invoking proof.rs lemmas.
Contains `View` impls mapping exec types to spec types.

### mod.rs

Module root. Either just re-exports submodules,
or contains exec code directly (as in `unbill-console-verified/settlement/mod.rs`
where the exec function lives in mod.rs alongside `pub mod proof; pub mod spec;`).

### Example: `unbill-model-verified/src/ledger/`

```
mod.rs    -- pub mod check; pub mod exec; pub mod proof; pub mod spec;
spec.rs   -- ShareSpec, BillSpec, LedgerStateSpec, total_weight,
              ledger_invariant, transition predicates, exec_*_requires predicates
proof.rs  -- seq_map_push, total_weight_push/nonneg/includes_each/partial_le,
              init_preserves, add_user_preserves, add_device_preserves, add_bill_preserves
exec.rs   -- Share, Bill, User, Device, LedgerState (Vec-based), View impls,
              exec_init/add_user/add_device/add_bill, try_add_user/device/bill,
              ledger_user_at/device_at/bill_at bridge lemmas
check.rs  -- error enums (AddUserError, AddDeviceError, AddBillError),
              user-friendly limits (MAX_USERS, MAX_BILLS, etc.),
              check_add_user/device/bill (proven: Ok => exec requires holds),
              containment helpers (user_id_in_ledger, device_in_ledger, bill_id_in_ledger)
```

### Example: `unbill-console-verified/src/settlement/`

```
mod.rs   -- pub mod proof; pub mod spec; + split_shares exec function
spec.rs  -- amount_sum, floor_amount, split_shares_requires, split_shares_ensures
proof.rs -- amount_sum lemmas, floor_sum lemmas, mod_distinct, floor_sum_eq_amount_sum
           (no exec.rs — the exec function lives in mod.rs)
```

## Why separate verified crates

Verus cannot process crates that use `dyn` trait objects.
`autosurgeon` generates `dyn Error`,
so production model crates cannot be compiled by Verus.

Verified crates isolate pure logic from the `dyn`-heavy dependency graph.
IDs are modeled as `Seq<u8>` (spec) / `Vec<u8>` (exec),
matching the production ULID string representation.
Production code maps domain types to/from verified types at the boundary.

### thiserror inside verified crates

`thiserror` is an optional dependency of the verified crate, gated behind a
`thiserror` feature flag. Error enums are defined inside `verus!{}` with
`#[cfg_attr(feature = "thiserror", derive(Debug, thiserror::Error))]` and
`#[cfg_attr(feature = "thiserror", error("..."))]` on each variant.

When the feature is off (during `cargo verus verify`), the attributes are
stripped and the enums are plain Verus types. When on (enabled by the
production `unbill-model` crate), the enums get `Debug + Display + Error`.

Workspace-level `cargo verus verify` does not work because dependents
enable the feature. Run Verus per-crate from each verified crate's directory.

## Cross-crate type sharing

The settlement module in `unbill-console-verified` reuses `Share`/`ShareSpec`
and `total_weight` lemmas from `unbill-model-verified`.

### Wildcard imports for erased items

Under `cargo build`, Verus spec/proof fns are erased from the compiled output.
Named imports of erased items (`use model::proof::{total_weight_push}`) fail with E0432.
Use wildcard imports (`use model::proof::*`) instead — they silently import nothing
when items are erased, and import everything under cargo-verus.

### Cross-crate View unfolding

Z3 cannot beta-reduce `shares@.map(|_i, s: Share| s@)` across crate boundaries.
The `open spec fn view()` trait impl from the model crate is not unfolded
by the SMT solver when used in the consumer crate.

The workaround is a local `open spec fn` using `Seq::new` with explicit
struct construction, bypassing the View trait entirely:

```
pub open spec fn shares_to_specs(shares: Seq<Share>) -> Seq<ShareSpec> {
    Seq::new(shares.len() as nat, |i: int|
        ShareSpec { user_id: shares[i].user_id@, weight: shares[i].weight }
    )
}
```

This gives Verus direct `Seq::new` axiom access for element properties
(`spec_shares[i].weight == shares@[i].weight` becomes trivially true).

## Contract pattern

Each public exec function has a single requires and ensures predicate,
named after the function:

```
requires spec::split_shares_requires(shares_to_specs(shares@), total_cents),
ensures  spec::split_shares_ensures(shares_to_specs(shares@), total_cents, result@),
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
4. Run `cargo verus verify` from each verified crate's directory.
   Workspace-level verification does not work when dependents enable the `thiserror` feature.
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

The Vec↔Seq bridge (mapping between runtime Vec and spec Seq)
is mechanical and tedious. Minimizing the gap between
spec types and exec types reduces this overhead.

Within-crate `Seq::map` closure reduction works (Z3 can beta-reduce),
but the bridge lemma pattern (`ledger_user_at`, `ledger_device_at`,
`ledger_bill_at`) is more reliable for connecting exec-level comparisons
to spec-level predicates like `has_user`.

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
- `Vec<T>@` gives `Seq<T>` not `Seq<T::V>`. Use `.map(|_i, s| s@)` or `Seq::new`.
- Prove `has_foo_true` and `has_foo_false` lemmas by contradiction or witness.
- Triggers cannot contain lambdas — trigger on `vec[k]` not `vec.map(f)[k]`.

### Cross-crate View does not unfold
Z3 cannot beta-reduce through `Seq::map` + lambda + cross-crate `View::view()`.
Use `Seq::new` with explicit field construction instead of `.map(|_i, s| s@)`.
See "Cross-crate View unfolding" above for the full pattern.

### Checker functions use bridge lemmas, not Seq::map
The containment helpers (`user_id_in_ledger`, etc.) work at the spec level
through `ledger_user_at`/`ledger_device_at`/`ledger_bill_at` bridge lemmas.
These connect `ledger@.users[i]` to `ledger.users@[i]@`, giving Z3 direct
access to exec-level field comparisons. Attempting to use `Seq::map` in
loop invariants or ensures within the same crate is unreliable — Z3 may
fail to reduce the closure application even when the View is `open`.

### Tighter checker bounds imply spec bounds
Checker functions use user-friendly limits (2M entries, ~$20M amounts,
1000 shares per side) that are strictly tighter than the mathematical
i32::MAX bounds in the spec. The proof that tighter bounds imply spec
bounds is straightforward arithmetic. For total_weight, per-share weight > 0
and bounded share count imply total_weight > 0 and total_weight <= i64::MAX
without exposing "total weight" to users.

### Types defined outside verus!{} cannot be used inside
Verus ignores types declared outside `verus!{}` or marked `#[verifier::external]`.
Error enums used in checker return types must stay inside `verus!{}`.
Use `#[cfg_attr(feature = "thiserror", ...)]` to conditionally attach
proc-macro derives that Verus cannot process.
