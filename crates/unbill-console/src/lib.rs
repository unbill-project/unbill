// unbill-console: the console-side library that projects ledger state for shells.
// See unbill-docs/unbill-console.md for the module structure and invariants.

pub mod conflict;
pub mod error;
pub use unbill_event as event;
pub use unbill_model as model;
pub use unbill_model::LedgerDoc;
pub mod service;
pub mod settlement;
pub mod storage;
