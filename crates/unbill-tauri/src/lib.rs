use std::collections::BTreeMap;
use std::sync::Arc;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tauri::{Manager, State};
use unbill_core::model::{
    BillId, Currency, LedgerId, NewBill, NewLedger, NewUser, NewUserName, NodeId, Share, UserId,
};
use unbill_core::service::UnbillService;
use unbill_core::storage::LockableStore;
use unbill_store_fs::{FsStore, LockedFsStore};

struct AppState {
    service: Arc<UnbillService<LockedFsStore>>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AppBootstrapDto {
    device_id: String,
    ledgers: Vec<LedgerSummaryDto>,
    all_users: Vec<UserDto>,
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
    devices: Vec<SyncDeviceDto>,
    bills: Vec<BillDto>,
    settlement: Vec<TransactionDto>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct TransactionDto {
    from_name: String,
    to_name: String,
    amount_cents: i64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UserDto {
    user_id: String,
    display_name: String,
    added_at_ms: i64,
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
    amount_cents: i64,
    description: String,
    created_at_ms: i64,
    payers: Vec<ShareDto>,
    payees: Vec<ShareDto>,
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
struct CreateUserInput {
    ledger_id: String,
    display_name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AddUserInput {
    ledger_id: String,
    user_id: String,
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
    amount_cents: i64,
    payers: Vec<BillShareInput>,
    payees: Vec<BillShareInput>,
    prev_bill_id: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PreviewBillSplitInput {
    bill_id: String,
    amount_cents: i64,
    payers: Vec<BillShareInput>,
    payees: Vec<BillShareInput>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct UserAmountDto {
    user_id: String,
    amount_cents: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BillSplitPreviewDto {
    payer_amounts: Vec<UserAmountDto>,
    payee_amounts: Vec<UserAmountDto>,
}

#[tauri::command]
async fn bootstrap_app(state: State<'_, AppState>) -> std::result::Result<AppBootstrapDto, String> {
    bootstrap_app_inner(&state.service)
        .await
        .map_err(stringify_error)
}

#[tauri::command]
async fn create_ledger(
    input: CreateLedgerInput,
    state: State<'_, AppState>,
) -> std::result::Result<LedgerSummaryDto, String> {
    let currency = Currency::from_code(&input.currency)
        .ok_or_else(|| format!("unknown currency code: {}", input.currency))?;
    let ledger_id = state
        .service
        .create_ledger(NewLedger {
            name: input.name,
            currency,
        })
        .await
        .map_err(stringify_error)?;

    load_ledger_detail_inner(&state.service, ledger_id)
        .await
        .map(|detail| detail.summary)
        .map_err(stringify_error)
}

#[tauri::command]
async fn load_ledger_detail(
    ledger_id: String,
    state: State<'_, AppState>,
) -> std::result::Result<LedgerDetailDto, String> {
    let lid = parse_ledger_id(&ledger_id).map_err(stringify_error)?;
    load_ledger_detail_inner(&state.service, lid)
        .await
        .map_err(stringify_error)
}

#[tauri::command]
async fn create_user(
    input: CreateUserInput,
    state: State<'_, AppState>,
) -> std::result::Result<UserDto, String> {
    let lid = parse_ledger_id(&input.ledger_id).map_err(stringify_error)?;
    state
        .service
        .create_user(
            lid,
            NewUserName {
                display_name: input.display_name,
            },
        )
        .await
        .map(UserDto::from)
        .map_err(stringify_error)
}

#[tauri::command]
async fn add_user(
    input: AddUserInput,
    state: State<'_, AppState>,
) -> std::result::Result<UserDto, String> {
    add_user_inner(&state.service, input)
        .await
        .map_err(stringify_error)
}

#[tauri::command]
async fn create_invitation(
    ledger_id: String,
    state: State<'_, AppState>,
) -> std::result::Result<String, String> {
    let lid = parse_ledger_id(&ledger_id).map_err(stringify_error)?;
    state
        .service
        .create_invitation(lid)
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
    let lid = parse_ledger_id(&input.ledger_id).map_err(stringify_error)?;

    let payers = input
        .payers
        .into_iter()
        .map(|item| {
            Ok(Share {
                user_id: parse_user_id(&item.user_id)?,
                shares: item.shares,
            })
        })
        .collect::<Result<Vec<_>>>()
        .map_err(stringify_error)?;

    let payees = input
        .payees
        .into_iter()
        .map(|item| {
            Ok(Share {
                user_id: parse_user_id(&item.user_id)?,
                shares: item.shares,
            })
        })
        .collect::<Result<Vec<_>>>()
        .map_err(stringify_error)?;

    let prev = input
        .prev_bill_id
        .into_iter()
        .map(|bill_id| parse_bill_id(&bill_id))
        .collect::<Result<Vec<_>>>()
        .map_err(stringify_error)?;

    let bill_id = state
        .service
        .add_bill(
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
        .map_err(stringify_error)?;

    Ok(bill_id.to_string())
}

#[tauri::command]
fn preview_bill_split(
    input: PreviewBillSplitInput,
) -> std::result::Result<BillSplitPreviewDto, String> {
    let bill_id = parse_bill_id(&input.bill_id).map_err(stringify_error)?;
    let payers = input
        .payers
        .into_iter()
        .map(|item| {
            Ok(Share {
                user_id: parse_user_id(&item.user_id)?,
                shares: item.shares,
            })
        })
        .collect::<Result<Vec<_>>>()
        .map_err(stringify_error)?;
    let payees = input
        .payees
        .into_iter()
        .map(|item| {
            Ok(Share {
                user_id: parse_user_id(&item.user_id)?,
                shares: item.shares,
            })
        })
        .collect::<Result<Vec<_>>>()
        .map_err(stringify_error)?;

    let split =
        unbill_core::settlement::compute_bill_split(&payers, &payees, input.amount_cents, bill_id);
    Ok(BillSplitPreviewDto {
        payer_amounts: split
            .payer_amounts
            .into_iter()
            .map(|(id, amount_cents)| UserAmountDto {
                user_id: id.to_string(),
                amount_cents,
            })
            .collect(),
        payee_amounts: split
            .payee_amounts
            .into_iter()
            .map(|(id, amount_cents)| UserAmountDto {
                user_id: id.to_string(),
                amount_cents,
            })
            .collect(),
    })
}

#[tauri::command]
async fn sync_once(
    peer_node_id: String,
    state: State<'_, AppState>,
) -> std::result::Result<(), String> {
    let peer = peer_node_id.parse::<NodeId>().map_err(stringify_error)?;
    state.service.sync_once(peer).await.map_err(stringify_error)
}

async fn load_ledgers<S>(service: &Arc<UnbillService<S>>) -> Result<Vec<LedgerSummaryDto>>
where
    S: LockableStore + Send + Sync + 'static,
    for<'a> S::Guard<'a>: Send,
{
    let metas = service.list_ledgers().await?;
    let mut summaries = Vec::with_capacity(metas.len());
    for meta in metas {
        summaries.push(summarize_ledger(service, meta).await?);
    }
    Ok(summaries)
}

async fn bootstrap_app_inner<S>(service: &Arc<UnbillService<S>>) -> Result<AppBootstrapDto>
where
    S: LockableStore + Send + Sync + 'static,
    for<'a> S::Guard<'a>: Send,
{
    let ledgers = load_ledgers(service).await?;
    let all_users = service
        .list_all_users()
        .await?
        .into_iter()
        .map(UserDto::from)
        .collect();
    let devices = load_sync_devices(service).await?;

    Ok(AppBootstrapDto {
        device_id: service.device_id().to_string(),
        ledgers,
        all_users,
        devices,
    })
}

async fn add_user_inner<S>(service: &Arc<UnbillService<S>>, input: AddUserInput) -> Result<UserDto>
where
    S: LockableStore + Send + Sync + 'static,
    for<'a> S::Guard<'a>: Send,
{
    let user_id = parse_user_id(&input.user_id)?;
    let lid = parse_ledger_id(&input.ledger_id)?;
    let existing = service
        .list_all_users()
        .await?
        .into_iter()
        .find(|user| user.user_id == user_id)
        .with_context(|| format!("user not found: {}", input.user_id))?;

    service
        .add_user(
            lid,
            NewUser {
                user_id,
                display_name: existing.display_name,
            },
        )
        .await?;

    let added_user = service
        .list_users(lid)
        .await?
        .into_iter()
        .find(|user| user.user_id == user_id)
        .context("new user missing after add")?;

    Ok(UserDto::from(added_user))
}

async fn load_sync_devices<S>(service: &Arc<UnbillService<S>>) -> Result<Vec<SyncDeviceDto>>
where
    S: LockableStore + Send + Sync + 'static,
    for<'a> S::Guard<'a>: Send,
{
    let local_node_id = service.device_id().to_string();
    let device_labels = service.list_device_labels().await?;
    let mut by_node_id = BTreeMap::<String, SyncDeviceDto>::new();

    for meta in service.list_ledgers().await? {
        let ledger_id = meta.ledger_id;
        let ledger_name = meta.name.clone();
        for device in load_sync_devices_for_ledger(
            service,
            ledger_id,
            &ledger_name,
            &local_node_id,
            &device_labels,
        )
        .await?
        {
            let entry = by_node_id
                .entry(device.node_id.clone())
                .or_insert_with(|| SyncDeviceDto {
                    node_id: device.node_id,
                    label: device.label,
                    ledger_names: Vec::new(),
                });
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

async fn load_sync_devices_for_ledger<S>(
    service: &Arc<UnbillService<S>>,
    ledger_id: LedgerId,
    ledger_name: &str,
    local_node_id: &str,
    device_labels: &std::collections::HashMap<String, String>,
) -> Result<Vec<SyncDeviceDto>>
where
    S: LockableStore + Send + Sync + 'static,
    for<'a> S::Guard<'a>: Send,
{
    let mut devices = service
        .list_devices(ledger_id)
        .await?
        .into_iter()
        .filter_map(|device| {
            let node_id = device.node_id.to_string();
            if node_id == local_node_id {
                return None;
            }

            Some(SyncDeviceDto {
                label: device_labels.get(&node_id).cloned().unwrap_or_default(),
                node_id,
                ledger_names: vec![ledger_name.to_owned()],
            })
        })
        .collect::<Vec<_>>();

    devices.sort_by(|left, right| {
        left.label
            .to_lowercase()
            .cmp(&right.label.to_lowercase())
            .then_with(|| left.node_id.cmp(&right.node_id))
    });
    Ok(devices)
}

async fn load_ledger_detail_inner<S>(
    service: &Arc<UnbillService<S>>,
    ledger_id: LedgerId,
) -> Result<LedgerDetailDto>
where
    S: LockableStore + Send + Sync + 'static,
    for<'a> S::Guard<'a>: Send,
{
    let meta = service
        .list_ledgers()
        .await?
        .into_iter()
        .find(|item| item.ledger_id == ledger_id)
        .with_context(|| format!("ledger {ledger_id} not found"))?;

    let summary = summarize_ledger(service, meta).await?;
    let local_node_id = service.device_id().to_string();
    let device_labels = service.list_device_labels().await?;
    let devices = load_sync_devices_for_ledger(
        service,
        ledger_id,
        &summary.name,
        &local_node_id,
        &device_labels,
    )
    .await?;
    let users = service.list_users(ledger_id).await?;
    let bills = service.list_bills(ledger_id).await?;

    let user_name_lookup: std::collections::HashMap<UserId, String> = users
        .iter()
        .map(|u| (u.user_id, u.display_name.clone()))
        .collect();

    let settlement = service
        .settle_ledger(ledger_id)
        .await?
        .transactions
        .into_iter()
        .map(|t| TransactionDto {
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

    let user_dtos = users.into_iter().map(UserDto::from).collect::<Vec<_>>();
    let bill_dtos = map_bills_from(bills, &user_name_lookup);

    Ok(LedgerDetailDto {
        summary,
        users: user_dtos,
        devices,
        bills: bill_dtos,
        settlement,
    })
}

async fn summarize_ledger<S>(
    service: &Arc<UnbillService<S>>,
    meta: unbill_core::model::LedgerMeta,
) -> Result<LedgerSummaryDto>
where
    S: LockableStore + Send + Sync + 'static,
    for<'a> S::Guard<'a>: Send,
{
    let users = service.list_users(meta.ledger_id).await?;
    let bills = service.list_bills(meta.ledger_id).await?;
    let latest_bill_at_ms = bills.iter().map(|bill| bill.created_at.as_millis()).max();

    Ok(LedgerSummaryDto {
        ledger_id: meta.ledger_id.to_string(),
        name: meta.name,
        currency: meta.currency.code().to_owned(),
        created_at_ms: meta.created_at.as_millis(),
        updated_at_ms: meta.updated_at.as_millis(),
        user_count: users.len(),
        latest_bill_at_ms,
    })
}

fn map_bills_from(
    bills: unbill_core::model::EffectiveBills,
    user_lookup: &std::collections::HashMap<UserId, String>,
) -> Vec<BillDto> {
    let mut items = bills
        .into_vec()
        .into_iter()
        .map(|bill| {
            let to_share_dto = |share: unbill_core::model::Share| ShareDto {
                display_name: user_lookup
                    .get(&share.user_id)
                    .cloned()
                    .unwrap_or_else(|| share.user_id.to_string()),
                user_id: share.user_id.to_string(),
                shares: share.shares,
            };
            BillDto {
                id: bill.id.to_string(),
                amount_cents: bill.amount_cents,
                description: bill.description,
                created_at_ms: bill.created_at.as_millis(),
                payers: bill.payers.into_iter().map(to_share_dto).collect(),
                payees: bill.payees.into_iter().map(to_share_dto).collect(),
                prev: bill.prev.into_iter().map(|prev| prev.to_string()).collect(),
            }
        })
        .collect::<Vec<_>>();

    items.sort_by_key(|item| std::cmp::Reverse(item.created_at_ms));
    items
}

fn parse_ledger_id(value: &str) -> Result<LedgerId> {
    LedgerId::from_string(value)
        .map_err(|error| anyhow::anyhow!("invalid ledger ID {value:?}: {error}"))
}

fn parse_user_id(value: &str) -> Result<UserId> {
    UserId::from_string(value)
        .map_err(|error| anyhow::anyhow!("invalid user ID {value:?}: {error}"))
}

fn parse_bill_id(value: &str) -> Result<BillId> {
    BillId::from_string(value)
        .map_err(|error| anyhow::anyhow!("invalid bill ID {value:?}: {error}"))
}

fn stringify_error(error: impl std::fmt::Display) -> String {
    error.to_string()
}

impl From<unbill_core::model::User> for UserDto {
    fn from(value: unbill_core::model::User) -> Self {
        Self {
            user_id: value.user_id.to_string(),
            display_name: value.display_name,
            added_at_ms: value.added_at.as_millis(),
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let _ = rustls::crypto::ring::default_provider().install_default();
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            #[cfg(mobile)]
            let root =
                app.path()
                    .app_data_dir()
                    .map_err(|error| -> Box<dyn std::error::Error> {
                        Box::new(std::io::Error::other(error.to_string()))
                    })?;
            #[cfg(not(mobile))]
            let root = unbill_store_fs::UNBILL_PATH.ensure_data_dir().map_err(
                |error| -> Box<dyn std::error::Error> {
                    Box::new(std::io::Error::other(error.to_string()))
                },
            )?;
            let store = Arc::new(LockedFsStore::new(
                FsStore::new(root.clone()),
                root.join(".unbill.lock"),
            ));
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
            create_user,
            add_user,
            create_invitation,
            join_ledger,
            save_bill,
            sync_once,
            preview_bill_split
        ])
        .run(tauri::generate_context!())
        .expect("error while running unbill");
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use serde_json::Value;
    use unbill_core::model::{NewDevice, UserId};
    use unbill_core::service::UnbillService;
    use unbill_store_memory::LockedInMemoryStore;

    fn tauri_config() -> Value {
        serde_json::from_str(include_str!("../tauri.conf.json"))
            .expect("tauri.conf.json should stay valid JSON")
    }

    #[test]
    fn desktop_config_points_to_leptos_frontend() {
        let config = tauri_config();
        let build = &config["build"];

        assert_eq!(build["devUrl"].as_str(), Some("http://localhost:1420"));
        assert_eq!(
            build["frontendDist"].as_str(),
            Some("../../apps/unbill-ui-native/dist")
        );

        let before_dev = &build["beforeDevCommand"];
        assert_eq!(
            before_dev["cwd"].as_str(),
            Some("../../apps/unbill-ui-native")
        );
        assert_eq!(
            before_dev["script"].as_str(),
            Some("trunk serve --config Trunk.toml")
        );
    }

    #[test]
    fn desktop_config_defines_visible_main_window() {
        let config = tauri_config();
        let windows = config["app"]["windows"]
            .as_array()
            .expect("desktop config should define at least one window");
        let main_window = windows
            .iter()
            .find(|window| window["label"].as_str() == Some("main"))
            .expect("desktop config should define a main window");

        assert_eq!(main_window["title"].as_str(), Some("unbill"));
        assert!(main_window["visible"].as_bool().unwrap_or(true));
    }

    #[tokio::test]
    async fn ledger_detail_includes_sync_devices_for_selected_ledger() {
        let host = UnbillService::open(Arc::new(LockedInMemoryStore::default()))
            .await
            .unwrap();
        let kitchen = UnbillService::open(Arc::new(LockedInMemoryStore::default()))
            .await
            .unwrap();
        let travel = UnbillService::open(Arc::new(LockedInMemoryStore::default()))
            .await
            .unwrap();

        let groceries = host
            .create_ledger(super::NewLedger {
                name: "Groceries".to_owned(),
                currency: super::Currency::from_code("USD").unwrap(),
            })
            .await
            .unwrap();
        let trip = host
            .create_ledger(super::NewLedger {
                name: "Trip".to_owned(),
                currency: super::Currency::from_code("USD").unwrap(),
            })
            .await
            .unwrap();

        host.add_device(
            groceries,
            NewDevice {
                node_id: kitchen.device_id(),
            },
        )
        .await
        .unwrap();
        host.add_device(
            trip,
            NewDevice {
                node_id: travel.device_id(),
            },
        )
        .await
        .unwrap();
        host.set_device_label(kitchen.device_id(), "Kitchen iPad".to_owned())
            .await
            .unwrap();

        let detail = super::load_ledger_detail_inner(&host, groceries)
            .await
            .unwrap();

        assert_eq!(detail.devices.len(), 1);
        assert_eq!(detail.devices[0].node_id, kitchen.device_id().to_string());
        assert_eq!(detail.devices[0].label, "Kitchen iPad");
        assert_eq!(detail.devices[0].ledger_names, vec!["Groceries"]);
    }

    #[tokio::test]
    async fn bootstrap_includes_current_device_id() {
        let service = UnbillService::open(Arc::new(LockedInMemoryStore::default()))
            .await
            .unwrap();

        let bootstrap = super::bootstrap_app_inner(&service).await.unwrap();

        assert_eq!(bootstrap.device_id, service.device_id().to_string());
    }

    #[tokio::test]
    async fn add_user_to_ledger_from_another_ledger_preserves_identity() {
        let service = UnbillService::open(Arc::new(LockedInMemoryStore::default()))
            .await
            .unwrap();
        let ledger_a = service
            .create_ledger(super::NewLedger {
                name: "Kitchen".to_owned(),
                currency: super::Currency::from_code("USD").unwrap(),
            })
            .await
            .unwrap();
        let ledger_b = service
            .create_ledger(super::NewLedger {
                name: "Travel".to_owned(),
                currency: super::Currency::from_code("USD").unwrap(),
            })
            .await
            .unwrap();
        let user = service
            .create_user(
                ledger_a,
                super::NewUserName {
                    display_name: "Mio".to_owned(),
                },
            )
            .await
            .unwrap();

        let added_user = super::add_user_inner(
            &service,
            super::AddUserInput {
                ledger_id: ledger_b.to_string(),
                user_id: user.user_id.to_string(),
            },
        )
        .await
        .unwrap();

        assert_eq!(added_user.user_id, user.user_id.to_string());
        assert_eq!(added_user.display_name, "Mio");
    }

    #[tokio::test]
    async fn add_user_with_unknown_id_fails_clearly() {
        let service = UnbillService::open(Arc::new(LockedInMemoryStore::default()))
            .await
            .unwrap();
        let ledger_id = service
            .create_ledger(super::NewLedger {
                name: "Kitchen".to_owned(),
                currency: super::Currency::from_code("USD").unwrap(),
            })
            .await
            .unwrap();

        let error = super::add_user_inner(
            &service,
            super::AddUserInput {
                ledger_id: ledger_id.to_string(),
                user_id: UserId::new().to_string(),
            },
        )
        .await
        .unwrap_err();

        assert!(error.to_string().contains("user not found"));
    }

    #[tokio::test]
    async fn bootstrap_all_users_aggregates_across_ledgers() {
        let service = UnbillService::open(Arc::new(LockedInMemoryStore::default()))
            .await
            .unwrap();
        let ledger_a = service
            .create_ledger(super::NewLedger {
                name: "Kitchen".to_owned(),
                currency: super::Currency::from_code("USD").unwrap(),
            })
            .await
            .unwrap();
        let ledger_b = service
            .create_ledger(super::NewLedger {
                name: "Travel".to_owned(),
                currency: super::Currency::from_code("USD").unwrap(),
            })
            .await
            .unwrap();
        service
            .create_user(
                ledger_a,
                super::NewUserName {
                    display_name: "Alice".to_owned(),
                },
            )
            .await
            .unwrap();
        service
            .create_user(
                ledger_b,
                super::NewUserName {
                    display_name: "Bob".to_owned(),
                },
            )
            .await
            .unwrap();

        let bootstrap = super::bootstrap_app_inner(&service).await.unwrap();

        assert_eq!(bootstrap.all_users.len(), 2);
    }
}
