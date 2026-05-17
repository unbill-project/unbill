// Persistence layer reexports. See unbill-docs/unbill-storage.md before changing it.
// unbill-storage is now a dev-dependency only; these reexports are test-only.

#[cfg(test)]
pub use unbill_storage::{LedgerStore, StorageResult};
