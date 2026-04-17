// Persistence layer. See storage/DESIGN.md before implementing.

mod fs;
mod memory;
mod traits;

pub use fs::FsStore;
pub use memory::InMemoryStore;
pub use traits::LedgerStore;
