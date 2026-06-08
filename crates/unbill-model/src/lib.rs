// Domain types. See unbill-docs/data-model.md for the schema and invariants.

// sirno:witness:unbill-model:begin
pub mod error;
pub use error::{AddBillOpError, AddDeviceOpError, AddUserOpError, StorageError, UnbillError};

mod amendment;
mod bill;
mod currency;
mod id;
mod invite_token;
mod node_id;
mod secret_key;
mod timestamp;
mod user;

mod doc;
pub(crate) mod ops;
pub mod verified_bridge;

pub use amendment::EffectiveBills;
pub use bill::{Bill, NewBill, Share};
pub use currency::Currency;
pub use doc::LedgerDoc;
pub use id::{BillId, LedgerId, UserId};
pub use invite_token::{InvalidInviteToken, InviteToken};
pub use node_id::NodeId;
pub use secret_key::SecretKey;
pub use timestamp::Timestamp;
pub use user::{
    Device, Invitation, Ledger, LedgerMeta, NewDevice, NewLedger, NewUser, NewUserName, User,
};
// sirno:witness:unbill-model:end
