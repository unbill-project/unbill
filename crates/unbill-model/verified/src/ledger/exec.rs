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

/// Convert a Seq<Share> to Seq<ShareSpec> without relying on cross-crate View unfolding.
/// Use this in exec function contracts instead of `shares@.map(|_i, s| s@)`.
pub open spec fn shares_to_specs(shares: Seq<Share>) -> Seq<spec::ShareSpec> {
    Seq::new(shares.len() as nat, |i: int|
        spec::ShareSpec { user_id: shares[i].user_id@, weight: shares[i].weight }
    )
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

// ---------------------------------------------------------------------------
// Exec operations
// ---------------------------------------------------------------------------

/// Create a fresh empty ledger.
pub fn exec_init(
    ledger_id: Vec<u8>,
    name: Vec<u8>,
    currency: Vec<u8>,
    created_at: i64,
) -> (result: LedgerState)
    ensures
        spec::init(result@, ledger_id@, name@, currency@, created_at),
        spec::ledger_invariant(result@),
{
    let result = LedgerState {
        ledger_id,
        schema_version: 1,
        name,
        currency,
        created_at,
        users: Vec::new(),
        bills: Vec::new(),
        devices: Vec::new(),
    };
    proof { super::proof::init_preserves(result@, result.ledger_id@, result.name@, result.currency@, created_at); }
    result
}

/// Add a user to a ledger.
pub fn exec_add_user(ledger: &mut LedgerState, user: User)
    requires
        spec::ledger_invariant(old(ledger)@),
        !spec::has_user(old(ledger)@.users, user@.user_id),
    ensures
        spec::add_user(old(ledger)@, final(ledger)@, user@),
        spec::ledger_invariant(final(ledger)@),
{
    let ghost pre = ledger@;
    ledger.users.push(user);
    proof {
        super::proof::seq_map_push::<User, spec::UserSpec>(old(ledger).users@, user, |_i: int, u: User| u@);
        assert(ledger@ =~= spec::LedgerStateSpec { users: pre.users.push(user@), ..pre });
        super::proof::add_user_preserves(pre, ledger@, user@);
    }
}

/// Add a device to a ledger.
pub fn exec_add_device(ledger: &mut LedgerState, device: Device)
    requires
        spec::ledger_invariant(old(ledger)@),
        !spec::has_device(old(ledger)@.devices, device@.node_id),
    ensures
        spec::add_device(old(ledger)@, final(ledger)@, device@),
        spec::ledger_invariant(final(ledger)@),
{
    let ghost pre = ledger@;
    ledger.devices.push(device);
    proof {
        super::proof::seq_map_push::<Device, spec::DeviceSpec>(old(ledger).devices@, device, |_i: int, d: Device| d@);
        assert(ledger@ =~= spec::LedgerStateSpec { devices: pre.devices.push(device@), ..pre });
        super::proof::add_device_preserves(pre, ledger@, device@);
    }
}

/// Add a bill to a ledger.
pub fn exec_add_bill(ledger: &mut LedgerState, bill: Bill)
    requires
        spec::ledger_invariant(old(ledger)@),
        spec::add_bill(old(ledger)@, spec::LedgerStateSpec { bills: old(ledger)@.bills.push(bill@), ..old(ledger)@ }, bill@),
    ensures
        spec::add_bill(old(ledger)@, final(ledger)@, bill@),
        spec::ledger_invariant(final(ledger)@),
{
    let ghost pre = ledger@;
    ledger.bills.push(bill);
    proof {
        super::proof::seq_map_push::<Bill, spec::BillSpec>(old(ledger).bills@, bill, |_i: int, b: Bill| b@);
        assert(ledger@ =~= spec::LedgerStateSpec { bills: pre.bills.push(bill@), ..pre });
        super::proof::add_bill_preserves(pre, ledger@, bill@);
    }
}

}
