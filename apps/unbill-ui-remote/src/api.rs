use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use unbill_core::model::{
    BillId, Currency, LedgerId, NewBill, NewLedger, NewUser, NewUserName, UserId,
};
use unbill_core::service::UnbillService;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;

thread_local! {
    static SERVICE: RefCell<Option<Arc<UnbillService>>> = const { RefCell::new(None) };
}

pub fn init(service: Arc<UnbillService>) {
    SERVICE.with(|cell| *cell.borrow_mut() = Some(service));
}

fn get_service() -> Result<Arc<UnbillService>, String> {
    SERVICE.with(|cell| {
        cell.borrow()
            .clone()
            .ok_or_else(|| "service not initialized".to_owned())
    })
}

// ---------------------------------------------------------------------------
// DTOs
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppBootstrap {
    pub device_id: String,
    pub ledgers: Vec<LedgerSummary>,
    pub all_users: Vec<User>,
    pub devices: Vec<SyncDevice>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LedgerSummary {
    pub ledger_id: String,
    pub name: String,
    pub currency: String,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
    pub user_count: usize,
    pub latest_bill_at_ms: Option<i64>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LedgerDetail {
    pub summary: LedgerSummary,
    pub users: Vec<User>,
    pub devices: Vec<SyncDevice>,
    pub bills: Vec<Bill>,
    pub settlement: Vec<Transaction>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Transaction {
    pub from_name: String,
    pub to_name: String,
    pub amount_cents: i64,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SyncDevice {
    pub node_id: String,
    pub label: String,
    pub ledger_names: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub user_id: String,
    pub display_name: String,
    pub added_at_ms: i64,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Share {
    pub user_id: String,
    pub shares: u32,
    pub display_name: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Bill {
    pub id: String,
    pub amount_cents: i64,
    pub description: String,
    pub created_at_ms: i64,
    pub payers: Vec<Share>,
    pub payees: Vec<Share>,
    pub prev: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateLedgerInput {
    pub name: String,
    pub currency: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateUserInput {
    pub ledger_id: String,
    pub display_name: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddUserInput {
    pub ledger_id: String,
    pub user_id: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveBillInput {
    pub ledger_id: String,
    pub description: String,
    pub amount_cents: i64,
    pub payers: Vec<BillShareInput>,
    pub payees: Vec<BillShareInput>,
    pub prev_bill_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BillShareInput {
    pub user_id: String,
    pub shares: u32,
}

// ---------------------------------------------------------------------------
// API functions
// ---------------------------------------------------------------------------

pub fn subscribe() -> tokio::sync::broadcast::Receiver<unbill_core::service::ServiceEvent> {
    get_service().expect("service not initialized").subscribe()
}

pub async fn load_ledger_summary(ledger_id: &str) -> Result<LedgerSummary, String> {
    let svc = get_service()?;
    let meta = svc
        .list_ledgers()
        .await
        .map_err(|e| e.to_string())?
        .into_iter()
        .find(|item| item.ledger_id.to_string() == ledger_id)
        .ok_or_else(|| format!("ledger {ledger_id} not found"))?;
    summarize_ledger(&svc, meta).await
}

pub async fn load_all_users() -> Result<Vec<User>, String> {
    let svc = get_service()?;
    svc.list_all_users()
        .await
        .map_err(|e| e.to_string())
        .map(|users| users.into_iter().map(user_to_dto).collect())
}

pub async fn bootstrap_app() -> Result<AppBootstrap, String> {
    let svc = get_service()?;
    let metas = svc.list_ledgers().await.map_err(|e| e.to_string())?;
    let devices = load_all_sync_devices(&svc, &metas).await?;
    let mut ledgers = Vec::with_capacity(metas.len());
    for meta in metas {
        ledgers.push(summarize_ledger(&svc, meta).await?);
    }
    let all_users = svc
        .list_all_users()
        .await
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(user_to_dto)
        .collect();
    Ok(AppBootstrap {
        device_id: svc.device_id().to_string(),
        ledgers,
        all_users,
        devices,
    })
}

pub async fn create_ledger(input: CreateLedgerInput) -> Result<LedgerSummary, String> {
    let svc = get_service()?;
    let currency = Currency::from_code(&input.currency)
        .ok_or_else(|| format!("unknown currency code: {}", input.currency))?;
    let ledger_id = svc
        .create_ledger(NewLedger {
            name: input.name,
            currency,
        })
        .await
        .map_err(|e| e.to_string())?;
    load_ledger_detail(&ledger_id.to_string())
        .await
        .map(|detail| detail.summary)
}

pub async fn load_ledger_detail(ledger_id: &str) -> Result<LedgerDetail, String> {
    let svc = get_service()?;
    let lid = parse_ledger_id(ledger_id)?;
    let meta = svc
        .list_ledgers()
        .await
        .map_err(|e| e.to_string())?
        .into_iter()
        .find(|item| item.ledger_id == lid)
        .ok_or_else(|| format!("ledger {ledger_id} not found"))?;

    let summary = summarize_ledger(&svc, meta).await?;
    let local_node_id = svc.device_id().to_string();
    let device_labels = svc.list_device_labels().await.map_err(|e| e.to_string())?;
    let devices = load_devices_for_ledger(&svc, lid, &local_node_id, &device_labels).await?;
    let users = svc.list_users(lid).await.map_err(|e| e.to_string())?;
    let bills = svc.list_bills(lid).await.map_err(|e| e.to_string())?;

    let user_name_lookup: HashMap<UserId, String> = users
        .iter()
        .map(|u| (u.user_id, u.display_name.clone()))
        .collect();

    let settlement = svc
        .settle_ledger(lid)
        .await
        .map_err(|e| e.to_string())?
        .transactions
        .into_iter()
        .map(|t| Transaction {
            from_name: user_name_lookup
                .get(&t.from_user_id)
                .cloned()
                .unwrap_or_else(|| t.from_user_id.to_string()),
            to_name: user_name_lookup
                .get(&t.to_user_id)
                .cloned()
                .unwrap_or_else(|| t.to_user_id.to_string()),
            amount_cents: t.amount_cents,
        })
        .collect();

    let user_dtos = users.into_iter().map(user_to_dto).collect();
    let bill_dtos = map_bills(bills, &user_name_lookup);

    Ok(LedgerDetail {
        summary,
        users: user_dtos,
        devices,
        bills: bill_dtos,
        settlement,
    })
}

pub async fn create_user(input: CreateUserInput) -> Result<User, String> {
    let svc = get_service()?;
    let lid = parse_ledger_id(&input.ledger_id)?;
    svc.create_user(
        lid,
        NewUserName {
            display_name: input.display_name,
        },
    )
    .await
    .map(user_to_dto)
    .map_err(|e| e.to_string())
}

pub async fn add_user(input: AddUserInput) -> Result<User, String> {
    let svc = get_service()?;
    let lid = parse_ledger_id(&input.ledger_id)?;
    let user_id = parse_user_id(&input.user_id)?;
    let existing = svc
        .list_all_users()
        .await
        .map_err(|e| e.to_string())?
        .into_iter()
        .find(|u| u.user_id == user_id)
        .ok_or_else(|| format!("user not found: {}", input.user_id))?;
    svc.add_user(
        lid,
        NewUser {
            user_id,
            display_name: existing.display_name,
        },
    )
    .await
    .map_err(|e| e.to_string())?;
    let added = svc
        .list_users(lid)
        .await
        .map_err(|e| e.to_string())?
        .into_iter()
        .find(|u| u.user_id == user_id)
        .ok_or_else(|| "new user missing after add".to_owned())?;
    Ok(user_to_dto(added))
}

pub async fn create_invitation(ledger_id: &str) -> Result<String, String> {
    let svc = get_service()?;
    svc.create_invitation(parse_ledger_id(ledger_id)?)
        .await
        .map_err(|e| e.to_string())
}

pub async fn join_ledger(url: String, label: String) -> Result<(), String> {
    let svc = get_service()?;
    svc.join_ledger(&url, label)
        .await
        .map_err(|e| e.to_string())
}

pub async fn sync_device(node_id: String) -> Result<(), String> {
    use std::str::FromStr;
    let svc = get_service()?;
    let peer = unbill_core::model::NodeId::from_str(&node_id)
        .map_err(|e| format!("invalid node id: {e}"))?;
    svc.sync_once(peer).await.map_err(|e| e.to_string())
}

pub async fn save_bill(input: SaveBillInput) -> Result<String, String> {
    let svc = get_service()?;
    let lid = parse_ledger_id(&input.ledger_id)?;
    let payers = input
        .payers
        .into_iter()
        .map(|item| {
            parse_user_id(&item.user_id).map(|user_id| unbill_core::model::Share {
                user_id,
                shares: item.shares,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let payees = input
        .payees
        .into_iter()
        .map(|item| {
            parse_user_id(&item.user_id).map(|user_id| unbill_core::model::Share {
                user_id,
                shares: item.shares,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let prev = input
        .prev_bill_id
        .into_iter()
        .map(|id| parse_bill_id(&id))
        .collect::<Result<Vec<_>, _>>()?;
    svc.add_bill(
        lid,
        NewBill {
            amount_cents: input.amount_cents,
            description: input.description,
            payers,
            payees,
            prev,
        },
    )
    .await
    .map(|id| id.to_string())
    .map_err(|e| e.to_string())
}

pub async fn write_clipboard_text(text: &str) -> Result<(), String> {
    let window = web_sys::window().ok_or("no browser window")?;
    let clipboard = window.navigator().clipboard();
    JsFuture::from(clipboard.write_text(text))
        .await
        .map(|_| ())
        .map_err(|e| {
            e.as_string()
                .unwrap_or_else(|| "clipboard write failed".to_owned())
        })
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

async fn load_all_sync_devices(
    svc: &Arc<UnbillService>,
    metas: &[unbill_core::model::LedgerMeta],
) -> Result<Vec<SyncDevice>, String> {
    use std::collections::BTreeMap;
    let local_node_id = svc.device_id().to_string();
    let device_labels = svc.list_device_labels().await.map_err(|e| e.to_string())?;
    let mut by_node_id: BTreeMap<String, SyncDevice> = BTreeMap::new();
    for meta in metas {
        let ledger_name = meta.name.clone();
        let devices = svc
            .list_devices(meta.ledger_id)
            .await
            .map_err(|e| e.to_string())?;
        for device in devices {
            let node_id = device.node_id.to_string();
            if node_id == local_node_id {
                continue;
            }
            if let Some(entry) = by_node_id.get_mut(&node_id) {
                entry.ledger_names.push(ledger_name.clone());
            } else {
                by_node_id.insert(
                    node_id.clone(),
                    SyncDevice {
                        label: device_labels.get(&node_id).cloned().unwrap_or_default(),
                        node_id,
                        ledger_names: vec![ledger_name.clone()],
                    },
                );
            }
        }
    }
    let mut devices: Vec<SyncDevice> = by_node_id.into_values().collect();
    devices.sort_by(|a, b| {
        a.label
            .to_lowercase()
            .cmp(&b.label.to_lowercase())
            .then_with(|| a.node_id.cmp(&b.node_id))
    });
    Ok(devices)
}

async fn summarize_ledger(
    svc: &Arc<UnbillService>,
    meta: unbill_core::model::LedgerMeta,
) -> Result<LedgerSummary, String> {
    let lid = meta.ledger_id;
    let users = svc.list_users(lid).await.map_err(|e| e.to_string())?;
    let bills = svc.list_bills(lid).await.map_err(|e| e.to_string())?;
    let latest_bill_at_ms = bills.iter().map(|bill| bill.created_at.as_millis()).max();
    Ok(LedgerSummary {
        ledger_id: lid.to_string(),
        name: meta.name,
        currency: meta.currency.code().to_owned(),
        created_at_ms: meta.created_at.as_millis(),
        updated_at_ms: meta.updated_at.as_millis(),
        user_count: users.len(),
        latest_bill_at_ms,
    })
}

async fn load_devices_for_ledger(
    svc: &Arc<UnbillService>,
    ledger_id: LedgerId,
    local_node_id: &str,
    device_labels: &HashMap<String, String>,
) -> Result<Vec<SyncDevice>, String> {
    let devices = svc
        .list_devices(ledger_id)
        .await
        .map_err(|e| e.to_string())?
        .into_iter()
        .filter_map(|device| {
            let node_id = device.node_id.to_string();
            if node_id == local_node_id {
                return None;
            }
            Some(SyncDevice {
                label: device_labels.get(&node_id).cloned().unwrap_or_default(),
                node_id,
                ledger_names: vec![],
            })
        })
        .collect();
    Ok(devices)
}

fn map_bills(
    bills: unbill_core::model::EffectiveBills,
    user_lookup: &HashMap<UserId, String>,
) -> Vec<Bill> {
    let mut items = bills
        .into_vec()
        .into_iter()
        .map(|bill| {
            let to_share = |share: unbill_core::model::Share| Share {
                display_name: user_lookup
                    .get(&share.user_id)
                    .cloned()
                    .unwrap_or_else(|| share.user_id.to_string()),
                user_id: share.user_id.to_string(),
                shares: share.shares,
            };
            Bill {
                id: bill.id.to_string(),
                amount_cents: bill.amount_cents,
                description: bill.description,
                created_at_ms: bill.created_at.as_millis(),
                payers: bill.payers.into_iter().map(to_share).collect(),
                payees: bill.payees.into_iter().map(to_share).collect(),
                prev: bill.prev.into_iter().map(|p| p.to_string()).collect(),
            }
        })
        .collect::<Vec<_>>();
    items.sort_by_key(|item| std::cmp::Reverse(item.created_at_ms));
    items
}

fn user_to_dto(user: unbill_core::model::User) -> User {
    User {
        user_id: user.user_id.to_string(),
        display_name: user.display_name,
        added_at_ms: user.added_at.as_millis(),
    }
}

fn parse_ledger_id(value: &str) -> Result<LedgerId, String> {
    LedgerId::from_string(value).map_err(|e| format!("invalid ledger ID {value:?}: {e}"))
}

fn parse_user_id(value: &str) -> Result<UserId, String> {
    UserId::from_string(value).map_err(|e| format!("invalid user ID {value:?}: {e}"))
}

fn parse_bill_id(value: &str) -> Result<BillId, String> {
    BillId::from_string(value).map_err(|e| format!("invalid bill ID {value:?}: {e}"))
}

pub fn format_money(amount_cents: i64, currency: &str) -> String {
    let sign = if amount_cents < 0 { "-" } else { "" };
    let absolute = amount_cents.abs();
    let units = absolute / 100;
    let cents = absolute % 100;
    format!("{sign}{currency} {units}.{cents:02}")
}

pub fn format_timestamp(timestamp_ms: i64) -> String {
    let date = js_sys::Date::new(&JsValue::from_f64(timestamp_ms as f64));
    format_timestamp_parts(
        date.get_full_year(),
        date.get_month() + 1,
        date.get_date(),
        date.get_hours(),
        date.get_minutes(),
    )
}

fn format_timestamp_parts(year: u32, month: u32, day: u32, hour: u32, minute: u32) -> String {
    format!("{year:04}/{month:02}/{day:02} {hour:02}:{minute:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timestamp_parts_render_as_zero_padded_local_date_and_time() {
        assert_eq!(format_timestamp_parts(2026, 5, 7, 9, 3), "2026/05/07 09:03");
        assert_eq!(
            format_timestamp_parts(2026, 11, 19, 23, 58),
            "2026/11/19 23:58"
        );
    }
}
