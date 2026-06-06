// Formally verified ledger model for unbill.
// This crate models the core ledger data structures and their invariants.
// Verified with Verus: `verus --crate-type lib crates/unbill-model/verified/src/lib.rs`
#![allow(clippy::ptr_arg, clippy::assign_op_pattern, deprecated, unused_imports)]

pub mod ledger0;
pub mod ledger1;
