#[cfg(not(target_arch = "wasm32"))]
pub mod path;
#[cfg(not(target_arch = "wasm32"))]
pub use path::{UNBILL_PATH, UnbillPath};

mod store;
mod store_server;

pub use store::{LedgerStore, StorageResult};
pub use store_server::StoreServer;
pub use unbill_model::LedgerDoc;
