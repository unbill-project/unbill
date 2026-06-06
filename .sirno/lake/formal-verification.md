---
core.name: Formal Verification
core.desc: The structure and tooling for formally verifying unbill's ledger invariants with Verus.
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
Settlement functions are parameterized over generic `Id` types.

Specifications live in a dedicated `spec.rs` file per module.
Each `spec.rs` contains only Verus specs:
`spec fn` definitions, `proof fn` lemmas, and ghost helpers.
No executable code, no tests, no data types.

Implementation files contain `exec fn` functions inside `verus!{}` blocks
with `requires`/`ensures` contracts and inline `proof {}` blocks for guidance.

```
crates/unbill-console/verified/src/settlement/
    mod.rs      -- exec functions with contracts inside verus!{}
    spec.rs     -- spec fn, proof fn lemmas
```

## What gets contracts

Only interface-level functions receive `requires`/`ensures`.
Internal helpers may carry contracts when needed for proof decomposition.

## Verification workflow

1. Write or update the contract and proof in the verified crate.
2. Run `verus crates/unbill-console/verified/src/lib.rs` to verify.
3. All verified code compiles with `cargo build` (verus!{} expands to valid Rust).
4. The main `unbill-console` crate calls the verified functions at runtime.
