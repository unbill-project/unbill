// Top-level service facade. Implementation begins at M2.
// See DESIGN.md §7 for the full public API.

mod service;

pub use service::{Identity, ServiceEvent, UnbillService};
