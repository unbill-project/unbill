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
The devenv packages it via a Nix derivation from the official release.
Verification runs as `verus <file.rs>` — separate from `cargo build`.

## Code structure

Verified code lives in `crates/unbill-console/verified/`.
This crate contains only pure arithmetic — no dyn, no thiserror, no external crate types.
It depends on `vstd` from crates.io,
which compiles as erased stubs under `cargo build`
and is fully verified under `verus`.
The `unbill-console` crate depends on `unbill-console-verified`
and calls the verified functions at runtime.

Each verified module has three files:

```
crates/unbill-console/verified/src/settlement/
    mod.rs      -- exec functions with requires/ensures contracts
    spec.rs     -- interface specs only: spec fn used in contracts
    proof.rs    -- proof fn lemmas and helper spec fn for proof guidance
```

`spec.rs` is kept short.
It defines only the `spec fn` that appear in `requires`/`ensures` clauses
of public functions.
No lemmas, no proof utilities, no executable code.

`proof.rs` contains proof utilities:
`proof fn` lemmas that establish properties of the spec functions,
and any helper `spec fn` needed only for proof decomposition.
These are never referenced from contracts — only from `proof {}` blocks.

`mod.rs` contains the `exec fn` implementations inside `verus!{}` blocks.
Contracts reference `spec.rs` definitions.
Inline `proof {}` blocks invoke lemmas from `proof.rs` to guide the solver.

## What gets contracts

Only interface-level functions receive `requires`/`ensures`.
The contracts reference only spec functions from `spec.rs`.
Internal helpers may carry contracts when needed for proof decomposition.

## Verification workflow

1. Define or update the specification in `spec.rs`.
2. Write or update proof lemmas in `proof.rs`.
3. Implement the function in `mod.rs` with contracts and proof blocks.
4. Run `verus --crate-type lib crates/unbill-console/verified/src/lib.rs`.
5. All verified code compiles with `cargo build` (`verus!{}` strips ghost code).
6. The main `unbill-console` crate calls the verified functions at runtime.
