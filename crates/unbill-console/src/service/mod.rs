// Top-level service facade. Implementation begins at M2.
// See DESIGN.md §7 for the full public API.

mod inner;

pub use crate::conflict::ConflictGroup;
pub use crate::settlement::{Settlement, Transaction as SettlementTransaction};
pub use inner::UnbillConsole;
pub use unbill_event::ServiceEvent;
