// Top-level service facade. Implementation begins at M2.
// See DESIGN.md §7 for the full public API.

mod inner;

pub use crate::conflict::ConflictGroup;
pub use inner::{LocalUser, ServiceEvent, UnbillService};
pub(crate) use inner::{
    load_device_labels, load_pending_invitations, load_pending_user_tokens, save_device_labels,
    save_pending_invitations, save_pending_user_tokens,
};
