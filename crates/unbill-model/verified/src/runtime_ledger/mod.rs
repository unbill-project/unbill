// runtime_ledger: Verified exec operations on runtime types.
// Uses ledger1::spec for the model and ledger1::proof for invariant preservation.
// Bridge lemmas in proof.rs connect runtime views to model predicates.

pub mod exec;
pub mod proof;
pub mod spec;
