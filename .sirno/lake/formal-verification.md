---
core.name: Formal Verification
core.desc: The structure and tooling for formally verifying unbill's ledger invariants with Creusot.
core.category:
  - core.meta
core.belongs:
  - unbill
core.refines:
  - development-workflow
---

Unbill uses Creusot for unbounded deductive verification of its core ledger algorithms.
Creusot translates annotated Rust to Why3 and proves contracts via SMT solvers (Z3, Alt-Ergo).

The properties to verify are defined in `formal-invariants`.

## Toolchain

`creusot-contracts` is a regular crate dependency.
Contracts (`#[requires]`, `#[ensures]`, `#[invariant]`) compile to no-ops under `cargo build`.
`cargo creusot` runs the verifier.

The devenv provides `cargo-creusot`, `creusot-rustc`, `why3`, and `z3`.

## Code structure

Specifications live in a dedicated `spec.rs` file per module.
Each `spec.rs` contains only Creusot specifications:
`#[logic]` functions, ghost helpers, and lemmas.
No executable code, no tests, no data types.

The implementation files (`mod.rs` or named files)
carry the `#[requires]` and `#[ensures]` attributes
on the public interface functions they define.
The contracts reference logic functions from `spec.rs`.

```
crates/unbill-console/src/settlement/
    mod.rs      -- implementation with contracts on public functions
    spec.rs     -- logic functions, ghost helpers, lemmas
```

This separation keeps proof machinery out of the production logic
and makes `spec.rs` the single place to read the formal specification
of a module's interface obligations.

## What gets contracts

Only interface-level functions receive `#[requires]`/`#[ensures]`:
functions that are public or called across module boundaries.
Internal helpers may carry contracts when needed for proof decomposition,
but the goal is to specify the observable interface.

## Verification workflow

1. Write or update the contract in `spec.rs` and on the function signature.
2. Run `cargo creusot` to generate Why3 obligations.
3. Discharge proofs automatically or interactively via Why3.
4. All contracts must pass `cargo build` as no-ops (no compilation breakage).
