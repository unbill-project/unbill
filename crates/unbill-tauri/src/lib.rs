use std::collections::BTreeMap;
use std::sync::Arc;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tauri::{Manager, State};
use unbill_core::model::{NewBill, NewUser, NodeId, Share, Ulid};
use unbill_core::path::UNBILL_PATH;
use unbill_core::service::{Identity, UnbillService};
use unbill_core::storage::FsStore;

struct AppState {
    service: Arc<UnbillService>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AppBootstrapDto {
    ledgers: Vec<LedgerSummaryDto>,
    identities: Vec<IdentityDto>,
    devices: Vec<SyncDeviceDto>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct LedgerSummaryDto {
    ledger_id: String,
    name: String,
    currency: String,
    created_at_ms: i64,
    updated_at_ms: i64,
    user_count: usize,
    latest_bill_at_ms: Option<i64>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct LedgerDetailDto {
    summary: LedgerSummaryDto,
    users: Vec<UserDto>,
    bills: Vec<BillDto>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct IdentityDto {
    user_id: String,
    display_name: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct UserDto {
    user_id: String,
    display_name: String,
    added_at_ms: i64,
    added_by: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ShareDto {
    user_id: String,
    shares: u32,
    display_name: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct BillDto {
    id: String,
    payer_user_id: String,
    payer_name: String,
    amount_cents: i64,
    description: String,
    created_at_ms: i64,
    shares: Vec<ShareDto>,
    prev: Vec<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SyncDeviceDto {
    node_id: String,
    label: String,
    ledger_names: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateLedgerInput {
    name: String,
    currency: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AddIdentityInput {
    display_name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AddUserInput {
    ledger_id: String,
    display_name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct JoinLedgerInput {
    url: String,
    label: String,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BillShareInput {
    user_id: String,
    shares: u32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SaveBillInput {
    ledger_id: String,
    description: String,
    payer_user_id: String,
    amount_cents: i64,
    shares: Vec<BillShareInput>,
    prev_bill_id: Option<String>,
}

#[tauri::command]
async fn bootstrap_app(state: State<'_, AppState>) -> std::result::Result<AppBootstrapDto, String> {
    let ledgers = load_ledgers(&state.service)
        .await
        .map_err(stringify_error)?;
    let identities = state
        .service
        .list_identities()
        .await
        .map(|items| items.into_iter().map(IdentityDto::from).collect())
        .map_err(stringify_error)?;
    let devices = load_sync_devices(&state.service)
        .await
        .map_err(stringify_error)?;

    Ok(AppBootstrapDto {
        ledgers,
        identities,
        devices,
    })
}

#[tauri::command]
async fn create_ledger(
    input: CreateLedgerInput,
    state: State<'_, AppState>,
) -> std::result::Result<LedgerSummaryDto, String> {
    let ledger_id = state
        .service
        .create_ledger(input.name, input.currency)
        .await
        .map_err(stringify_error)?;

    load_ledger_detail_inner(&state.service, &ledger_id)
        .await
        .map(|detail| detail.summary)
        .map_err(stringify_error)
}

#[tauri::command]
async fn load_ledger_detail(
    ledger_id: String,
    state: State<'_, AppState>,
) -> std::result::Result<LedgerDetailDto, String> {
    load_ledger_detail_inner(&state.service, &ledger_id)
        .await
        .map_err(stringify_error)
}

#[tauri::command]
async fn add_identity(
    input: AddIdentityInput,
    state: State<'_, AppState>,
) -> std::result::Result<IdentityDto, String> {
    state
        .service
        .add_identity(input.display_name)
        .await
        .map(IdentityDto::from)
        .map_err(stringify_error)
}

#[tauri::command]
async fn add_user(
    input: AddUserInput,
    state: State<'_, AppState>,
) -> std::result::Result<UserDto, String> {
    let existing_users = state
        .service
        .list_users(&input.ledger_id)
        .await
        .map_err(stringify_error)?;

    let user_id = Ulid::new();
    let added_by = existing_users
        .first()
        .map(|user| user.user_id)
        .unwrap_or(user_id);

    state
        .service
        .add_user(
            &input.ledger_id,
            NewUser {
                user_id,
                display_name: input.display_name,
                added_by,
            },
        )
        .await
        .map_err(stringify_error)?;

    let added_user = state
        .service
        .list_users(&input.ledger_id)
        .await
        .map_err(stringify_error)?
        .into_iter()
        .find(|user| user.user_id == user_id)
        .context("new user missing after add")
        .map_err(stringify_error)?;

    Ok(UserDto::from(added_user))
}

#[tauri::command]
async fn create_invitation(
    ledger_id: String,
    state: State<'_, AppState>,
) -> std::result::Result<String, String> {
    state
        .service
        .create_invitation(&ledger_id)
        .await
        .map_err(stringify_error)
}

#[tauri::command]
async fn join_ledger(
    input: JoinLedgerInput,
    state: State<'_, AppState>,
) -> std::result::Result<(), String> {
    state
        .service
        .join_ledger(&input.url, input.label)
        .await
        .map_err(stringify_error)
}

#[tauri::command]
async fn save_bill(
    input: SaveBillInput,
    state: State<'_, AppState>,
) -> std::result::Result<String, String> {
    let shares = input
        .shares
        .into_iter()
        .map(|item| {
            Ok(Share {
                user_id: parse_ulid(&item.user_id)?,
                shares: item.shares,
            })
        })
        .collect::<Result<Vec<_>>>()
        .map_err(stringify_error)?;

    let prev = input
        .prev_bill_id
        .into_iter()
        .map(|bill_id| parse_ulid(&bill_id))
        .collect::<Result<Vec<_>>>()
        .map_err(stringify_error)?;

    let bill_id = state
        .service
        .add_bill(
            &input.ledger_id,
            NewBill {
                payer_user_id: parse_ulid(&input.payer_user_id).map_err(stringify_error)?,
                amount_cents: input.amount_cents,
                description: input.description,
                shares,
                prev,
            },
        )
        .await
        .map_err(stringify_error)?;

    Ok(bill_id)
}

#[tauri::command]
async fn sync_once(
    peer_node_id: String,
    state: State<'_, AppState>,
) -> std::result::Result<(), String> {
    let peer = peer_node_id.parse::<NodeId>().map_err(stringify_error)?;
    state.service.sync_once(peer).await.map_err(stringify_error)
}

fn load_store_root() -> Result<std::path::PathBuf> {
    UNBILL_PATH.ensure_data_dir()
}

async fn load_ledgers(service: &Arc<UnbillService>) -> Result<Vec<LedgerSummaryDto>> {
    let metas = service.list_ledgers().await?;
    let mut summaries = Vec::with_capacity(metas.len());
    for meta in metas {
        summaries.push(summarize_ledger(service, meta).await?);
    }
    Ok(summaries)
}

async fn load_sync_devices(service: &Arc<UnbillService>) -> Result<Vec<SyncDeviceDto>> {
    let local_node_id = service.device_id().to_string();
    let mut by_node_id = BTreeMap::<String, SyncDeviceDto>::new();

    for meta in service.list_ledgers().await? {
        let ledger_id = meta.ledger_id.to_string();
        let ledger_name = meta.name.clone();
        for device in service.list_devices(&ledger_id).await? {
            let node_id = device.node_id.to_string();
            if node_id == local_node_id {
                continue;
            }

            let entry = by_node_id
                .entry(node_id.clone())
                .or_insert_with(|| SyncDeviceDto {
                    node_id,
                    label: device.label.clone(),
                    ledger_names: Vec::new(),
                });

            if entry.label.is_empty() && !device.label.is_empty() {
                entry.label = device.label.clone();
            }
            if !entry.ledger_names.iter().any(|name| name == &ledger_name) {
                entry.ledger_names.push(ledger_name.clone());
            }
        }
    }

    let mut devices = by_node_id.into_values().collect::<Vec<_>>();
    devices.sort_by(|left, right| {
        left.label
            .to_lowercase()
            .cmp(&right.label.to_lowercase())
            .then_with(|| left.node_id.cmp(&right.node_id))
    });
    Ok(devices)
}

async fn load_ledger_detail_inner(
    service: &Arc<UnbillService>,
    ledger_id: &str,
) -> Result<LedgerDetailDto> {
    let meta = service
        .list_ledgers()
        .await?
        .into_iter()
        .find(|item| item.ledger_id.to_string() == ledger_id)
        .with_context(|| format!("ledger {ledger_id} not found"))?;

    let summary = summarize_ledger(service, meta).await?;
    let users = service
        .list_users(ledger_id)
        .await?
        .into_iter()
        .map(UserDto::from)
        .collect::<Vec<_>>();
    let bills = map_bills(service, ledger_id).await?;

    Ok(LedgerDetailDto {
        summary,
        users,
        bills,
    })
}

async fn summarize_ledger(
    service: &Arc<UnbillService>,
    meta: unbill_core::model::LedgerMeta,
) -> Result<LedgerSummaryDto> {
    let ledger_id = meta.ledger_id.to_string();
    let users = service.list_users(&ledger_id).await?;
    let bills = service.list_bills(&ledger_id).await?;
    let latest_bill_at_ms = bills.iter().map(|bill| bill.created_at.as_millis()).max();

    Ok(LedgerSummaryDto {
        ledger_id,
        name: meta.name,
        currency: meta.currency.code().to_owned(),
        created_at_ms: meta.created_at.as_millis(),
        updated_at_ms: meta.updated_at.as_millis(),
        user_count: users.len(),
        latest_bill_at_ms,
    })
}

async fn map_bills(service: &Arc<UnbillService>, ledger_id: &str) -> Result<Vec<BillDto>> {
    let users = service.list_users(ledger_id).await?;
    let user_lookup = users
        .iter()
        .map(|user| (user.user_id, user.display_name.clone()))
        .collect::<std::collections::HashMap<_, _>>();
    let bills = service.list_bills(ledger_id).await?;

    let mut items = bills
        .into_vec()
        .into_iter()
        .map(|bill| BillDto {
            id: bill.id.to_string(),
            payer_user_id: bill.payer_user_id.to_string(),
            payer_name: user_lookup
                .get(&bill.payer_user_id)
                .cloned()
                .unwrap_or_else(|| bill.payer_user_id.to_string()),
            amount_cents: bill.amount_cents,
            description: bill.description,
            created_at_ms: bill.created_at.as_millis(),
            shares: bill
                .shares
                .into_iter()
                .map(|share| ShareDto {
                    display_name: user_lookup
                        .get(&share.user_id)
                        .cloned()
                        .unwrap_or_else(|| share.user_id.to_string()),
                    user_id: share.user_id.to_string(),
                    shares: share.shares,
                })
                .collect(),
            prev: bill.prev.into_iter().map(|prev| prev.to_string()).collect(),
        })
        .collect::<Vec<_>>();

    items.sort_by(|left, right| right.created_at_ms.cmp(&left.created_at_ms));
    Ok(items)
}

fn parse_ulid(value: &str) -> Result<Ulid> {
    Ulid::from_string(value).map_err(|error| anyhow::anyhow!("invalid ULID {value:?}: {error}"))
}

fn stringify_error(error: impl std::fmt::Display) -> String {
    error.to_string()
}

impl From<Identity> for IdentityDto {
    fn from(value: Identity) -> Self {
        Self {
            user_id: value.user_id.to_string(),
            display_name: value.display_name,
        }
    }
}

impl From<unbill_core::model::User> for UserDto {
    fn from(value: unbill_core::model::User) -> Self {
        Self {
            user_id: value.user_id.to_string(),
            display_name: value.display_name,
            added_at_ms: value.added_at.as_millis(),
            added_by: value.added_by.to_string(),
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let root = load_store_root().map_err(|error| -> Box<dyn std::error::Error> {
                Box::new(std::io::Error::other(error.to_string()))
            })?;
            let store = Arc::new(FsStore::new(root));
            let service = tauri::async_runtime::block_on(UnbillService::open(store)).map_err(
                |error| -> Box<dyn std::error::Error> {
                    Box::new(std::io::Error::other(error.to_string()))
                },
            )?;

            let accept_loop_service = Arc::clone(&service);
            tauri::async_runtime::spawn(async move {
                if let Err(error) = accept_loop_service.accept_loop().await {
                    tracing::error!("accept loop stopped: {error}");
                }
            });

            app.manage(AppState { service });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            bootstrap_app,
            create_ledger,
            load_ledger_detail,
            add_identity,
            add_user,
            create_invitation,
            join_ledger,
            save_bill,
            sync_once
        ])
        .run(tauri::generate_context!())
        .expect("error while running unbill");
}
