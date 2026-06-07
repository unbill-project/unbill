// Formally verified settlement math for unbill.
// This crate contains only pure arithmetic — verified with Verus.
// Run: `verus --crate-type lib crates/unbill-console/verified/src/lib.rs`
//
// Clippy lints are suppressed because Verus requires specific code patterns
// (e.g. `a = a + b` instead of `a += b`, `&Vec` instead of `&[_]`).
#![allow(
    clippy::ptr_arg,
    clippy::assign_op_pattern,
    deprecated,
    unused_imports,
    unused_variables
)]

pub use unbill_model_verified::ledger::exec::Share;

pub mod balance;
pub mod settlement;
