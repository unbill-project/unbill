// Formally verified ledger model for unbill.
// Three layers: LedgerStateModel → DeviceStateModel → WorldModel.
// Verified with Verus: `verus --crate-type lib crates/unbill-model/verified/src/lib.rs`
#![allow(
    clippy::ptr_arg,
    clippy::assign_op_pattern,
    clippy::len_zero,
    deprecated,
    unused_imports
)]

pub mod ledger;
