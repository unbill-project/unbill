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

Verification: `verus --crate-type lib crates/unbill-console/verified/src/lib.rs`

The verified crate depends on `vstd` from crates.io.
Under `cargo build`, `verus!{}` expands to valid Rust with ghost code erased.
Under `verus`, the full verification runs.
The `unbill-console` crate depends on `unbill-console-verified` and calls the verified functions at runtime.

## Crate edition

The verified crate uses `edition = "2021"`, not the workspace's `edition = "2024"`.
This is required because `vstd` is compiled for edition 2021.
Verus's `verus!{}` macro may not support 2024 syntax.

## Code structure

Each verified module has three files:

```
crates/unbill-console/verified/src/settlement/
    mod.rs      -- exec functions with requires/ensures contracts
    spec.rs     -- interface specs only: spec fn used in contracts
    proof.rs    -- proof fn lemmas and helper spec fn for proof guidance
```

`spec.rs` is kept short.
It defines only the `spec fn` that appear in `requires`/`ensures` clauses
of public functions: `amount_sum`, `spec_total_weight`, `floor_amount`,
`split_shares_requires`, `split_shares_ensures`.
No lemmas, no proof utilities, no executable code.

`proof.rs` contains proof utilities:
`proof fn` lemmas that establish properties of the spec functions,
and any helper `spec fn` needed only for proof decomposition.
These are never referenced from contracts — only from `proof {}` blocks.

`mod.rs` contains the `exec fn` implementations inside `verus!{}` blocks.
Contracts reference `spec.rs` definitions.
Inline `proof {}` blocks invoke lemmas from `proof.rs` to guide the solver.

## Why a separate crate

Verus cannot process crates that use `dyn` trait objects.
The `std::error::Error` trait requires `dyn` in its `source()` method,
so any crate using `thiserror` or `autosurgeon` (which generates `dyn Error`)
cannot be compiled by Verus.

The verified crate isolates pure arithmetic from the `dyn`-heavy dependency graph.
Functions are parameterized over `u64` IDs instead of `UserId`/`BillId`.
The `unbill-console` wrapper maps domain types to/from the verified types.

## Clippy

The verified crate suppresses clippy lints at the crate level:
`ptr_arg` (Verus requires `&Vec` not `&[_]`),
`assign_op_pattern` (Verus requires `a = a + b` not `a += b`),
`deprecated` (vstd's `SliceAdditionalExecFns::set`),
`unused_imports` (vstd imports used only by Verus, erased under cargo).

## Contract pattern

Each public function has a single `requires` predicate and a single `ensures` predicate,
both named after the function:

```
requires spec::split_shares_requires(shares@, total_cents),
ensures  spec::split_shares_ensures(shares@, total_cents, result@),
```

The predicates bundle all clauses. This keeps the function signature clean
and makes the specification easy to find in `spec.rs`.

## Verification workflow

1. Define or update the specification in `spec.rs`.
2. Write or update proof lemmas in `proof.rs`.
3. Implement the function in `mod.rs` with contracts and proof blocks.
4. Run `verus --crate-type lib crates/unbill-console/verified/src/lib.rs`.
5. All verified code compiles with `cargo build` (`verus!{}` strips ghost code).
6. Run `cargo test -p unbill-console` to confirm integration.

## Lessons learned

### Verus requires explicit loop structure
Iterator chains (`iter().map().sum()`) have no contracts in vstd.
Loops must be explicit `while` with `invariant` and `decreases` clauses.

### `nonlinear_arith` is essential for multiplication/division
Z3's default solver cannot reason about products and quotients.
Use `assert(...) by(nonlinear_arith) requires ...;` to dispatch
nonlinear goals to a specialized solver.
Keep the `requires` minimal — only the facts the nonlinear solver needs.

### `ext_eq` (`=~=`) for sequence equalities
When proving two `Seq` values are equal (e.g., `s.push(x).drop_last() =~= s`),
use extensional equality. The solver cannot derive this from axioms alone.

### Modular arithmetic needs manual case analysis
Z3 struggles with `%`. Prove modular properties by:
1. Using `vstd::arithmetic::div_mod::lemma_fundamental_div_mod` to decompose `x = q*d + r`.
2. Using `lemma_fundamental_div_mod_converse_mod` to establish `r == x % d`.
3. Case-splitting on whether `b + r < n` or `b + r >= n`.
The `mod_distinct` lemma (distinct indices under modular wrap) required
explicit case analysis across four cases.

### Floor division bounds require multiplicative reasoning
To prove `floor(a*b/c) <= a` when `b <= c`:
1. Prove `a*b <= a*c` via `nonlinear_arith`.
2. Use `vstd::arithmetic::div_mod::lemma_div_is_ordered` for monotonicity.
3. Use `lemma_div_multiples_vanish` for `(c*a)/c == a`.

### Sum-of-floors requires a proportional bound
To prove `Σ floor(t*w_i/W) <= t`:
1. Prove the stronger `floor_sum * W <= t * Σw_i` by induction.
2. Each step uses `floor_last * W <= t * w_last` (from floor definition).
3. When `Σw_i == W`, conclude `floor_sum <= t`.

To prove remainder `< n`:
1. Prove `t * Σw_i - floor_sum * W < n * W` (each floor loses `< W`).
2. Divide by `W > 0` to get `t - floor_sum < n`.

### Track ghost state for the remainder loop
The remainder distribution loop needs ghost state to prove fairness:
- `floor_amounts`: snapshot of amounts before remainder distribution.
- Per-element invariant: each amount is either `floor_amounts[j].1` or `floor_amounts[j].1 + 1`.
- "Unvisited" invariant: indices not yet touched still equal their floor value.
- The `mod_distinct` lemma proves the current index was not previously visited.

### Overflow requires preconditions, not runtime checks
Verus checks every arithmetic operation for overflow.
Adding `requires total_cents <= i32::MAX, shares.len() <= i32::MAX`
allows proving `total_cents * weight <= i32::MAX * u32::MAX < i64::MAX`
via `nonlinear_arith`. Track the bound through loop invariants.
The product `total_cents * (k+1)` chain needs explicit step-by-step assertions
because `nonlinear_arith` can't handle long chains.

### Connecting runtime sums to spec sums
The `floor_sum_eq_amount_sum` lemma bridges the gap between:
- `amount_sum(amounts@)` — the recursive spec sum over the Vec's view.
- `floor_sum(shares@, t, W)` — the recursive spec sum over floor divisions.
When each `amounts[j].1 == floor(t * shares[j].1 / W)` (tracked as a loop invariant),
these two sums are equal. Proved by induction over `drop_last`.
