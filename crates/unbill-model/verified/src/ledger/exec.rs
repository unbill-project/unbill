// Layer 1: Runtime types mirroring production Ledger/Bill/User/Device/Share.
// These types are callable by production code via `ledger_to_model` / `model_to_ledger`.

use super::spec;
use vstd::prelude::*;

verus! {

// ---------------------------------------------------------------------------
// Runtime types (Vec-based, mirrors production structs)
// ---------------------------------------------------------------------------

pub struct Share {
    pub user_id: Vec<u8>,
    pub weight: u32,
}

pub struct Bill {
    pub id: Vec<u8>,
    pub amount_cents: i64,
    pub description: Vec<u8>,
    pub payers: Vec<Share>,
    pub payees: Vec<Share>,
    pub prev: Vec<Vec<u8>>,
    pub created_at: i64,
    pub created_by_device: Vec<u8>,
}

pub struct User {
    pub user_id: Vec<u8>,
    pub display_name: Vec<u8>,
    pub added_at: i64,
}

pub struct Device {
    pub node_id: Vec<u8>,
    pub added_at: i64,
}

pub struct LedgerState {
    pub ledger_id: Vec<u8>,
    pub schema_version: u32,
    pub name: Vec<u8>,
    pub currency: Vec<u8>,
    pub created_at: i64,
    pub users: Vec<User>,
    pub bills: Vec<Bill>,
    pub devices: Vec<Device>,
}

// ---------------------------------------------------------------------------
// View impls: runtime → spec
// ---------------------------------------------------------------------------

impl View for Share {
    type V = spec::ShareSpec;
    open spec fn view(&self) -> spec::ShareSpec {
        spec::ShareSpec { user_id: self.user_id@, weight: self.weight }
    }
}

impl View for User {
    type V = spec::UserSpec;
    open spec fn view(&self) -> spec::UserSpec {
        spec::UserSpec {
            user_id: self.user_id@,
            display_name: self.display_name@,
            added_at: self.added_at,
        }
    }
}

impl View for Device {
    type V = spec::DeviceSpec;
    open spec fn view(&self) -> spec::DeviceSpec {
        spec::DeviceSpec {
            node_id: self.node_id@,
            added_at: self.added_at,
        }
    }
}

impl View for Bill {
    type V = spec::BillSpec;
    open spec fn view(&self) -> spec::BillSpec {
        spec::BillSpec {
            id: self.id@,
            amount_cents: self.amount_cents,
            description: self.description@,
            payers: self.payers@.map(|_i, s: Share| s@),
            payees: self.payees@.map(|_i, s: Share| s@),
            prev: self.prev@.map(|_i, v: Vec<u8>| v@),
            created_at: self.created_at,
            created_by_device: self.created_by_device@,
        }
    }
}

impl View for LedgerState {
    type V = spec::LedgerStateSpec;
    open spec fn view(&self) -> spec::LedgerStateSpec {
        spec::LedgerStateSpec {
            ledger_id: self.ledger_id@,
            schema_version: self.schema_version,
            name: self.name@,
            currency: self.currency@,
            created_at: self.created_at,
            users: self.users@.map(|_i, u: User| u@),
            bills: self.bills@.map(|_i, b: Bill| b@),
            devices: self.devices@.map(|_i, d: Device| d@),
        }
    }
}

}
