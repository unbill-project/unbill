// Top-level service facade. See unbill-docs/console-service.md for the API shape.

mod inner;

pub use crate::conflict::ConflictGroup;
pub use crate::settlement::{Settlement, Transaction as SettlementTransaction};
pub use inner::UnbillConsole;
pub use unbill_event::ServiceEvent;
