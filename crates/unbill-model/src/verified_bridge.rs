// Bridge between production Ledger types and the verified LedgerState model.
// Conversion functions for mapping to/from verified types.

use unbill_model_verified::ledger::exec as v;

use crate::bill::{Bill, Share};
use crate::user::{Device, Ledger, User};

// ---------------------------------------------------------------------------
// Production → Verified model
// ---------------------------------------------------------------------------

fn share_to_model(share: &Share) -> v::Share {
    v::Share {
        user_id: share.user_id.to_string().into_bytes(),
        weight: share.shares,
    }
}

fn user_to_model(user: &User) -> v::User {
    v::User {
        user_id: user.user_id.to_string().into_bytes(),
        display_name: user.display_name.clone().into_bytes(),
        added_at: user.added_at.as_millis(),
    }
}

fn device_to_model(device: &Device) -> v::Device {
    v::Device {
        node_id: device.node_id.to_string().into_bytes(),
        added_at: device.added_at.as_millis(),
    }
}

fn bill_to_model(bill: &Bill) -> v::Bill {
    v::Bill {
        id: bill.id.to_string().into_bytes(),
        amount_cents: bill.amount_cents,
        description: bill.description.clone().into_bytes(),
        payers: bill.payers.iter().map(share_to_model).collect(),
        payees: bill.payees.iter().map(share_to_model).collect(),
        prev: bill
            .prev
            .iter()
            .map(|id| id.to_string().into_bytes())
            .collect(),
        created_at: bill.created_at.as_millis(),
        created_by_device: bill.created_by_device.to_string().into_bytes(),
    }
}

/// Convert a production Ledger to a verified LedgerState.
pub fn ledger_to_model(ledger: &Ledger) -> v::LedgerState {
    v::LedgerState {
        ledger_id: ledger.ledger_id.to_string().into_bytes(),
        schema_version: ledger.schema_version,
        name: ledger.name.clone().into_bytes(),
        currency: ledger.currency.code().as_bytes().to_vec(),
        created_at: ledger.created_at.as_millis(),
        users: ledger.users.iter().map(user_to_model).collect(),
        bills: ledger.bills.iter().map(bill_to_model).collect(),
        devices: ledger.devices.iter().map(device_to_model).collect(),
    }
}

// ---------------------------------------------------------------------------
// Verified model → Production
// ---------------------------------------------------------------------------

use crate::currency::Currency;
use crate::id::{BillId, LedgerId, UserId};
use crate::node_id::NodeId;
use crate::timestamp::Timestamp;

/// Error returned when a verified model cannot be converted back to production types.
#[derive(Debug)]
pub struct BridgeError(pub String);

fn bytes_to_string(bytes: &[u8]) -> Result<String, BridgeError> {
    String::from_utf8(bytes.to_vec()).map_err(|e| BridgeError(format!("invalid UTF-8: {e}")))
}

fn model_to_share(share: &v::Share) -> Result<Share, BridgeError> {
    let id_str = bytes_to_string(&share.user_id)?;
    Ok(Share {
        user_id: UserId::from_string(&id_str).map_err(|e| BridgeError(format!("user_id: {e}")))?,
        shares: share.weight,
    })
}

fn model_to_user(user: &v::User) -> Result<User, BridgeError> {
    let id_str = bytes_to_string(&user.user_id)?;
    Ok(User {
        user_id: UserId::from_string(&id_str).map_err(|e| BridgeError(format!("user_id: {e}")))?,
        display_name: bytes_to_string(&user.display_name)?,
        added_at: Timestamp::from_millis(user.added_at),
    })
}

fn model_to_device(device: &v::Device) -> Result<Device, BridgeError> {
    let id_str = bytes_to_string(&device.node_id)?;
    Ok(Device {
        node_id: id_str
            .parse::<NodeId>()
            .map_err(|e| BridgeError(format!("node_id: {e}")))?,
        added_at: Timestamp::from_millis(device.added_at),
    })
}

fn model_to_bill(bill: &v::Bill) -> Result<Bill, BridgeError> {
    let id_str = bytes_to_string(&bill.id)?;
    let device_str = bytes_to_string(&bill.created_by_device)?;
    Ok(Bill {
        id: BillId::from_string(&id_str).map_err(|e| BridgeError(format!("bill_id: {e}")))?,
        amount_cents: bill.amount_cents,
        description: bytes_to_string(&bill.description)?,
        payers: bill
            .payers
            .iter()
            .map(model_to_share)
            .collect::<Result<_, _>>()?,
        payees: bill
            .payees
            .iter()
            .map(model_to_share)
            .collect::<Result<_, _>>()?,
        prev: bill
            .prev
            .iter()
            .map(|id| {
                let s = bytes_to_string(id)?;
                BillId::from_string(&s).map_err(|e| BridgeError(format!("prev bill_id: {e}")))
            })
            .collect::<Result<_, _>>()?,
        created_at: Timestamp::from_millis(bill.created_at),
        created_by_device: device_str
            .parse::<NodeId>()
            .map_err(|e| BridgeError(format!("created_by_device: {e}")))?,
    })
}

/// Convert a verified LedgerState back to a production Ledger.
/// Fails if any ID cannot be parsed (should not happen if the model
/// was produced by `ledger_to_model`).
pub fn model_to_ledger(model: &v::LedgerState) -> Result<Ledger, BridgeError> {
    let ledger_id_str = bytes_to_string(&model.ledger_id)?;
    let currency_str = bytes_to_string(&model.currency)?;
    Ok(Ledger {
        ledger_id: LedgerId::from_string(&ledger_id_str)
            .map_err(|e| BridgeError(format!("ledger_id: {e}")))?,
        schema_version: model.schema_version,
        name: bytes_to_string(&model.name)?,
        currency: Currency::from_code(&currency_str)
            .ok_or_else(|| BridgeError(format!("unknown currency: {currency_str}")))?,
        created_at: Timestamp::from_millis(model.created_at),
        users: model
            .users
            .iter()
            .map(model_to_user)
            .collect::<Result<_, _>>()?,
        bills: model
            .bills
            .iter()
            .map(model_to_bill)
            .collect::<Result<_, _>>()?,
        devices: model
            .devices
            .iter()
            .map(model_to_device)
            .collect::<Result<_, _>>()?,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BillId, Currency, LedgerId, NodeId, Timestamp, UserId};

    fn ts(ms: i64) -> Timestamp {
        Timestamp::from_millis(ms)
    }

    fn uid(n: u128) -> UserId {
        UserId::from_u128(n)
    }

    fn lid(n: u128) -> LedgerId {
        LedgerId::from_u128(n)
    }

    #[test]
    fn test_round_trip_empty_ledger() {
        let ledger = Ledger {
            ledger_id: lid(1),
            schema_version: 1,
            name: "Test".into(),
            currency: Currency::from_code("USD").unwrap(),
            created_at: ts(1000),
            users: vec![],
            bills: vec![],
            devices: vec![],
        };
        let model = ledger_to_model(&ledger);
        let recovered = model_to_ledger(&model).unwrap();
        assert_eq!(recovered.ledger_id, ledger.ledger_id);
        assert_eq!(recovered.name, ledger.name);
        assert_eq!(recovered.currency.code(), ledger.currency.code());
        assert_eq!(
            recovered.created_at.as_millis(),
            ledger.created_at.as_millis()
        );
        assert_eq!(recovered.schema_version, ledger.schema_version);
    }

    #[test]
    fn test_round_trip_with_users_and_bills() {
        let ledger = Ledger {
            ledger_id: lid(1),
            schema_version: 1,
            name: "Trip".into(),
            currency: Currency::from_code("EUR").unwrap(),
            created_at: ts(0),
            users: vec![
                User {
                    user_id: uid(10),
                    display_name: "Alice".into(),
                    added_at: ts(100),
                },
                User {
                    user_id: uid(20),
                    display_name: "Bob".into(),
                    added_at: ts(200),
                },
            ],
            bills: vec![Bill {
                id: BillId::from_u128(99),
                amount_cents: 5000,
                description: "Dinner".into(),
                payers: vec![Share {
                    user_id: uid(10),
                    shares: 1,
                }],
                payees: vec![
                    Share {
                        user_id: uid(10),
                        shares: 1,
                    },
                    Share {
                        user_id: uid(20),
                        shares: 1,
                    },
                ],
                prev: vec![],
                created_at: ts(300),
                created_by_device: NodeId::from_seed(1),
            }],
            devices: vec![Device {
                node_id: NodeId::from_seed(1),
                added_at: ts(50),
            }],
        };
        let model = ledger_to_model(&ledger);
        let recovered = model_to_ledger(&model).unwrap();
        assert_eq!(recovered.ledger_id, ledger.ledger_id);
        assert_eq!(recovered.users.len(), 2);
        assert_eq!(recovered.users[0].user_id, uid(10));
        assert_eq!(recovered.users[0].display_name, "Alice");
        assert_eq!(recovered.users[1].user_id, uid(20));
        assert_eq!(recovered.bills.len(), 1);
        assert_eq!(recovered.bills[0].id, BillId::from_u128(99));
        assert_eq!(recovered.bills[0].amount_cents, 5000);
        assert_eq!(recovered.bills[0].payees.len(), 2);
        assert_eq!(recovered.devices.len(), 1);
        assert_eq!(recovered.devices[0].node_id, NodeId::from_seed(1));
    }

    #[test]
    fn test_empty_ledger_round_trips() {
        let ledger = Ledger {
            ledger_id: lid(1),
            schema_version: 1,
            name: "Test".into(),
            currency: Currency::from_code("USD").unwrap(),
            created_at: ts(1000),
            users: vec![],
            bills: vec![],
            devices: vec![],
        };
        let model = ledger_to_model(&ledger);
        assert_eq!(model.schema_version, 1);
        assert_eq!(model.name, b"Test");
        assert_eq!(model.currency, b"USD");
        assert_eq!(model.created_at, 1000);
        assert!(model.users.is_empty());
        assert!(model.bills.is_empty());
        assert!(model.devices.is_empty());
    }

    #[test]
    fn test_ledger_with_users_maps_ids() {
        let ledger = Ledger {
            ledger_id: lid(1),
            schema_version: 1,
            name: "Test".into(),
            currency: Currency::from_code("EUR").unwrap(),
            created_at: ts(0),
            users: vec![
                User {
                    user_id: uid(10),
                    display_name: "Alice".into(),
                    added_at: ts(100),
                },
                User {
                    user_id: uid(20),
                    display_name: "Bob".into(),
                    added_at: ts(200),
                },
            ],
            bills: vec![],
            devices: vec![],
        };
        let model = ledger_to_model(&ledger);
        assert_eq!(model.users.len(), 2);
        assert_eq!(model.users[0].user_id, uid(10).to_string().into_bytes());
        assert_eq!(model.users[0].display_name, b"Alice");
        assert_eq!(model.users[0].added_at, 100);
        assert_eq!(model.users[1].user_id, uid(20).to_string().into_bytes());
    }

    #[test]
    fn test_ledger_with_bill_maps_shares() {
        let ledger = Ledger {
            ledger_id: lid(1),
            schema_version: 1,
            name: "Test".into(),
            currency: Currency::from_code("USD").unwrap(),
            created_at: ts(0),
            users: vec![User {
                user_id: uid(1),
                display_name: "Alice".into(),
                added_at: ts(0),
            }],
            bills: vec![Bill {
                id: BillId::from_u128(99),
                amount_cents: 5000,
                description: "Dinner".into(),
                payers: vec![Share {
                    user_id: uid(1),
                    shares: 1,
                }],
                payees: vec![Share {
                    user_id: uid(1),
                    shares: 1,
                }],
                prev: vec![],
                created_at: ts(500),
                created_by_device: NodeId::from_seed(1),
            }],
            devices: vec![],
        };
        let model = ledger_to_model(&ledger);
        assert_eq!(model.bills.len(), 1);
        assert_eq!(model.bills[0].amount_cents, 5000);
        assert_eq!(model.bills[0].description, b"Dinner");
        assert_eq!(model.bills[0].payers.len(), 1);
        assert_eq!(model.bills[0].payers[0].weight, 1);
        assert_eq!(
            model.bills[0].payers[0].user_id,
            uid(1).to_string().into_bytes()
        );
    }
}
