use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use tauri::{Emitter, Manager, State};
use unbill_asymmetric_channel::AsymChannel;
#[cfg(mobile)]
use unbill_asymmetric_channel::local::LocalAsymChannel;
#[cfg(not(mobile))]
use unbill_asymmetric_channel::rpc::RpcAsymChannel;
use unbill_console::error::{Result, UnbillError};
use unbill_console::model::{
    BillId, Currency, LedgerId, NewBill, NewLedger, NewUser, NewUserName, NodeId, Share, UserId,
};
use unbill_console::service::UnbillConsole;
#[cfg(mobile)]
use unbill_store_fs::FsStore;

// sirno:witness:unbill-tauri:begin
struct AppState {
    service: Arc<UnbillConsole>,
}

#[derive(Default)]
struct PendingDeepLinks(Mutex<Vec<String>>);

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
    user_names: Vec<String>,
    latest_bill_at_ms: Option<i64>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct LedgerDetailDto {
    summary: LedgerSummaryDto,
    users: Vec<UserDto>,
    devices: Vec<SyncDeviceDto>,
    bills: Vec<BillDto>,
    conflicts: Vec<ConflictGroupDto>,
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
struct ConflictGroupDto {
    conflicting: Vec<BillDto>,
    ancestors: Vec<BillDto>,
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
    label: Option<String>,
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
struct ResolveConflictInput {
    ledger_id: String,
    selected_bill_id: String,
    conflicting_bill_ids: Vec<String>,
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
// sirno:witness:unbill-tauri:end

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct UpdateInfoDto {
    version: String,
}

#[tauri::command]
async fn check_update(app: tauri::AppHandle) -> std::result::Result<Option<UpdateInfoDto>, String> {
    #[cfg(windows)]
    {
        use tauri_plugin_updater::UpdaterExt;
        match app.updater().map_err(|e| e.to_string())?.check().await {
            Ok(Some(update)) => Ok(Some(UpdateInfoDto {
                version: update.version.clone(),
            })),
            Ok(None) => Ok(None),
            Err(e) => {
                tracing::warn!("update check failed: {e}");
                Ok(None)
            }
        }
    }
    #[cfg(not(windows))]
    {
        let _ = app;
        Ok(None)
    }
}

#[tauri::command]
async fn install_update(app: tauri::AppHandle) -> std::result::Result<(), String> {
    #[cfg(windows)]
    {
        use tauri_plugin_updater::UpdaterExt;
        let update = app
            .updater()
            .map_err(|e| e.to_string())?
            .check()
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "no update available".to_owned())?;
        update
            .download_and_install(|_, _| {}, || {})
            .await
            .map_err(|e| e.to_string())?;
        app.restart();
    }
    #[cfg(not(windows))]
    {
        let _ = app;
        Ok(())
    }
}

// sirno:witness:unbill-tauri:begin
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
async fn resolve_conflict(
    input: ResolveConflictInput,
    state: State<'_, AppState>,
) -> std::result::Result<String, String> {
    resolve_conflict_inner(&state.service, input)
        .await
        .map_err(stringify_error)
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

    let split = unbill_console::settlement::compute_bill_split(
        &payers,
        &payees,
        input.amount_cents,
        bill_id,
    );
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
// sirno:witness:unbill-tauri:end

// sirno:witness:unbill-ui-native:begin
async fn load_ledgers(service: &Arc<UnbillConsole>) -> Result<Vec<LedgerSummaryDto>> {
    let metas = service.list_ledgers().await?;
    let mut summaries = Vec::with_capacity(metas.len());
    for meta in metas {
        summaries.push(summarize_ledger(service, meta).await?);
    }
    Ok(summaries)
}

async fn bootstrap_app_inner(service: &Arc<UnbillConsole>) -> Result<AppBootstrapDto> {
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

async fn add_user_inner(service: &Arc<UnbillConsole>, input: AddUserInput) -> Result<UserDto> {
    let user_id = parse_user_id(&input.user_id)?;
    let lid = parse_ledger_id(&input.ledger_id)?;
    let existing = service
        .list_all_users()
        .await?
        .into_iter()
        .find(|user| user.user_id == user_id)
        .ok_or_else(|| UnbillError::UserNotFound(input.user_id.clone()))?;

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
        .ok_or_else(|| UnbillError::UserNotFound("new user missing after add".into()))?;

    Ok(UserDto::from(added_user))
}

async fn load_sync_devices(service: &Arc<UnbillConsole>) -> Result<Vec<SyncDeviceDto>> {
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

async fn load_sync_devices_for_ledger(
    service: &Arc<UnbillConsole>,
    ledger_id: LedgerId,
    ledger_name: &str,
    local_node_id: &str,
    device_labels: &std::collections::HashMap<String, String>,
) -> Result<Vec<SyncDeviceDto>> {
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

async fn load_ledger_detail_inner(
    service: &Arc<UnbillConsole>,
    ledger_id: LedgerId,
) -> Result<LedgerDetailDto> {
    let meta = service
        .list_ledgers()
        .await?
        .into_iter()
        .find(|item| item.ledger_id == ledger_id)
        .ok_or_else(|| UnbillError::LedgerNotFound(ledger_id.to_string()))?;

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
    let conflicts = service.detect_conflicts(ledger_id).await?;

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
    let conflict_dtos = map_conflicts_from(conflicts, &user_name_lookup);

    Ok(LedgerDetailDto {
        summary,
        users: user_dtos,
        devices,
        bills: bill_dtos,
        conflicts: conflict_dtos,
        settlement,
    })
}

async fn resolve_conflict_inner(
    service: &Arc<UnbillConsole>,
    input: ResolveConflictInput,
) -> Result<String> {
    let ledger_id = parse_ledger_id(&input.ledger_id)?;
    let selected_bill_id = parse_bill_id(&input.selected_bill_id)?;
    let requested_conflicting_ids = input
        .conflicting_bill_ids
        .iter()
        .map(|id| parse_bill_id(id))
        .collect::<Result<BTreeSet<_>>>()?;

    if !requested_conflicting_ids.contains(&selected_bill_id) {
        return Err(UnbillError::Validation(
            "selected bill is not part of the conflict".into(),
        ));
    }

    let conflicts = service.detect_conflicts(ledger_id).await?;
    let group = conflicts
        .into_iter()
        .find(|group| {
            group
                .conflicting
                .iter()
                .map(|bill| bill.id)
                .collect::<BTreeSet<_>>()
                == requested_conflicting_ids
        })
        .ok_or_else(|| UnbillError::Validation("conflict group is no longer current".into()))?;

    let selected_bill = group
        .conflicting
        .into_iter()
        .find(|bill| bill.id == selected_bill_id)
        .ok_or_else(|| {
            UnbillError::Validation("selected bill is no longer a current conflicting bill".into())
        })?;

    let merge_id = service
        .add_bill(
            ledger_id,
            NewBill {
                amount_cents: selected_bill.amount_cents,
                description: selected_bill.description,
                payers: selected_bill.payers,
                payees: selected_bill.payees,
                prev: requested_conflicting_ids.into_iter().collect(),
            },
        )
        .await?;

    Ok(merge_id.to_string())
}

async fn summarize_ledger(
    service: &Arc<UnbillConsole>,
    meta: unbill_console::model::LedgerMeta,
) -> Result<LedgerSummaryDto> {
    let users = service.list_users(meta.ledger_id).await?;
    let bills = service.list_bills(meta.ledger_id).await?;
    let latest_bill_at_ms = bills.iter().map(|bill| bill.created_at.as_millis()).max();

    let user_names = users.iter().map(|u| u.display_name.clone()).collect();

    Ok(LedgerSummaryDto {
        ledger_id: meta.ledger_id.to_string(),
        name: meta.name,
        currency: meta.currency.code().to_owned(),
        created_at_ms: meta.created_at.as_millis(),
        updated_at_ms: meta.updated_at.as_millis(),
        user_count: users.len(),
        user_names,
        latest_bill_at_ms,
    })
}

fn map_bills_from(
    bills: unbill_console::model::EffectiveBills,
    user_lookup: &std::collections::HashMap<UserId, String>,
) -> Vec<BillDto> {
    let mut items = bills
        .into_vec()
        .into_iter()
        .map(|bill| bill_to_dto(bill, user_lookup))
        .collect::<Vec<_>>();

    items.sort_by_key(|item| std::cmp::Reverse(item.created_at_ms));
    items
}

fn map_conflicts_from(
    conflicts: Vec<unbill_console::service::ConflictGroup>,
    user_lookup: &std::collections::HashMap<UserId, String>,
) -> Vec<ConflictGroupDto> {
    conflicts
        .into_iter()
        .map(|group| ConflictGroupDto {
            conflicting: map_bill_vec(group.conflicting, user_lookup),
            ancestors: map_bill_vec(group.ancestors, user_lookup),
        })
        .collect()
}

fn map_bill_vec(
    bills: Vec<unbill_console::model::Bill>,
    user_lookup: &std::collections::HashMap<UserId, String>,
) -> Vec<BillDto> {
    let mut items = bills
        .into_iter()
        .map(|bill| bill_to_dto(bill, user_lookup))
        .collect::<Vec<_>>();
    items.sort_by_key(|item| std::cmp::Reverse(item.created_at_ms));
    items
}

fn bill_to_dto(
    bill: unbill_console::model::Bill,
    user_lookup: &std::collections::HashMap<UserId, String>,
) -> BillDto {
    let to_share_dto = |share: unbill_console::model::Share| ShareDto {
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
}

fn parse_ledger_id(value: &str) -> Result<LedgerId> {
    LedgerId::from_string(value).map_err(|source| UnbillError::ParseId {
        value: value.to_owned(),
        source,
    })
}

fn parse_user_id(value: &str) -> Result<UserId> {
    UserId::from_string(value).map_err(|source| UnbillError::ParseId {
        value: value.to_owned(),
        source,
    })
}

fn parse_bill_id(value: &str) -> Result<BillId> {
    BillId::from_string(value).map_err(|source| UnbillError::ParseId {
        value: value.to_owned(),
        source,
    })
}

fn stringify_error(error: impl std::fmt::Display) -> String {
    error.to_string()
}
// sirno:witness:unbill-ui-native:end

impl From<unbill_console::model::User> for UserDto {
    fn from(value: unbill_console::model::User) -> Self {
        Self {
            user_id: value.user_id.to_string(),
            display_name: value.display_name,
            added_at_ms: value.added_at.as_millis(),
        }
    }
}

fn extract_unbill_urls(urls: &[url::Url]) -> Vec<String> {
    urls.iter()
        .map(|u| u.to_string())
        .filter(|s| s.starts_with("unbill://"))
        .collect()
}

fn emit_deep_link_url(app: &tauri::AppHandle, url: &str) {
    let _ = app.emit("deep-link-open", url.to_owned());
}

#[tauri::command]
fn drain_pending_deep_links(pending: State<'_, PendingDeepLinks>) -> Vec<String> {
    std::mem::take(&mut pending.0.lock().unwrap())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // sirno:witness:unbill-tauri:begin
    let _ = rustls::crypto::ring::default_provider().install_default();
    let mut builder = tauri::Builder::default();

    #[cfg(desktop)]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_focus();
            }
            for arg in args {
                if arg.starts_with("unbill://") {
                    emit_deep_link_url(app, &arg);
                }
            }
        }));
    }

    builder
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_deep_link::init())
        .setup(|app| {
            #[cfg(windows)]
            app.handle()
                .plugin(tauri_plugin_updater::Builder::new().build())?;

            app.manage(PendingDeepLinks::default());

            // Register deep link schemes at dev time on Linux/Windows.
            #[cfg(any(target_os = "linux", all(debug_assertions, windows)))]
            {
                use tauri_plugin_deep_link::DeepLinkExt;
                let _ = app.deep_link().register_all();
            }

            // Store launch URLs so the frontend can retrieve them after mount.
            {
                use tauri_plugin_deep_link::DeepLinkExt;
                if let Ok(Some(urls)) = app.deep_link().get_current() {
                    let unbill_urls = extract_unbill_urls(&urls);
                    if !unbill_urls.is_empty() {
                        let pending: State<'_, PendingDeepLinks> = app.state();
                        pending.0.lock().unwrap().extend(unbill_urls);
                    }
                }
            }

            // Emit events for URLs arriving while the app is running.
            {
                use tauri_plugin_deep_link::DeepLinkExt;
                let handle = app.handle().clone();
                app.deep_link().on_open_url(move |event| {
                    let urls = event.urls();
                    for url in extract_unbill_urls(&urls) {
                        emit_deep_link_url(&handle, &url);
                    }
                });
            }

            #[cfg(mobile)]
            let service = tauri::async_runtime::block_on(async {
                let root = app
                    .path()
                    .app_data_dir()
                    .map_err(|e| std::io::Error::other(e.to_string()))?;
                let store = Arc::new(FsStore::open(root)?);
                let channel = LocalAsymChannel::open(store)
                    .await
                    .map_err(|e| std::io::Error::other(e.to_string()))?;
                let accept = Arc::clone(&channel);
                tauri::async_runtime::spawn(async move {
                    if let Err(e) = accept.accept_loop().await {
                        tracing::error!("accept loop stopped: {e}");
                    }
                });
                Ok::<_, std::io::Error>(UnbillConsole::open(channel as Arc<dyn AsymChannel>).await)
            })
            .map_err(|e| -> Box<dyn std::error::Error> { Box::new(e) })?;

            #[cfg(not(mobile))]
            let service = tauri::async_runtime::block_on(async {
                let socket = unbill_store_fs::UNBILL_PATH
                    .socket_path()
                    .map_err(|e| std::io::Error::other(e.to_string()))?;
                let channel = RpcAsymChannel::connect(&socket).await?;
                Ok::<_, std::io::Error>(UnbillConsole::open(channel as Arc<dyn AsymChannel>).await)
            })
            .map_err(|e| -> Box<dyn std::error::Error> { Box::new(e) })?;

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
            resolve_conflict,
            sync_once,
            preview_bill_split,
            check_update,
            install_update,
            drain_pending_deep_links
        ])
        .run(tauri::generate_context!())
        .expect("error while running unbill");
    // sirno:witness:unbill-tauri:end
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use serde_json::Value;
    use unbill_asymmetric_channel::AsymChannel;
    use unbill_asymmetric_channel::local::LocalAsymChannel;
    use unbill_console::model::{NewDevice, UserId};
    use unbill_console::service::UnbillConsole;
    use unbill_store_memory::InMemoryStore;

    async fn open_console() -> Arc<UnbillConsole> {
        let channel = LocalAsymChannel::open(Arc::new(InMemoryStore::default()))
            .await
            .unwrap();
        UnbillConsole::open(channel as Arc<dyn AsymChannel>).await
    }

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
        let host = open_console().await;
        let kitchen = open_console().await;
        let travel = open_console().await;

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
    async fn ledger_detail_includes_conflicting_bills_and_ancestors() {
        let service = open_console().await;
        let (ledger_id, _base_id, left_id, right_id) = create_conflicted_ledger(&service).await;

        let detail = super::load_ledger_detail_inner(&service, ledger_id)
            .await
            .unwrap();

        assert_eq!(detail.conflicts.len(), 1);
        let group = &detail.conflicts[0];
        let conflicting_ids = group
            .conflicting
            .iter()
            .map(|bill| bill.id.clone())
            .collect::<Vec<_>>();
        assert!(conflicting_ids.contains(&left_id.to_string()));
        assert!(conflicting_ids.contains(&right_id.to_string()));
        assert_eq!(group.ancestors.len(), 1);
    }

    #[tokio::test]
    async fn resolving_conflict_keeps_selected_bill_and_clears_group() {
        let service = open_console().await;
        let (ledger_id, _base_id, left_id, right_id) = create_conflicted_ledger(&service).await;

        let merge_id = super::resolve_conflict_inner(
            &service,
            super::ResolveConflictInput {
                ledger_id: ledger_id.to_string(),
                selected_bill_id: right_id.to_string(),
                conflicting_bill_ids: vec![left_id.to_string(), right_id.to_string()],
            },
        )
        .await
        .unwrap();

        let conflicts = service.detect_conflicts(ledger_id).await.unwrap();
        assert!(conflicts.is_empty());
        let bills = service.list_bills(ledger_id).await.unwrap().into_vec();
        let merge_bill = bills
            .iter()
            .find(|bill| bill.id.to_string() == merge_id)
            .unwrap();
        assert_eq!(merge_bill.description, "Right branch");
        assert_eq!(merge_bill.amount_cents, 1500);
        assert!(merge_bill.prev.contains(&left_id));
        assert!(merge_bill.prev.contains(&right_id));
    }

    #[tokio::test]
    async fn bootstrap_includes_current_device_id() {
        let service = open_console().await;

        let bootstrap = super::bootstrap_app_inner(&service).await.unwrap();

        assert_eq!(bootstrap.device_id, service.device_id().to_string());
    }

    #[tokio::test]
    async fn add_user_to_ledger_from_another_ledger_preserves_identity() {
        let service = open_console().await;
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
        let service = open_console().await;
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
        let service = open_console().await;
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

    async fn create_conflicted_ledger(
        service: &Arc<UnbillConsole>,
    ) -> (super::LedgerId, super::BillId, super::BillId, super::BillId) {
        let ledger_id = service
            .create_ledger(super::NewLedger {
                name: "Kitchen".to_owned(),
                currency: super::Currency::from_code("USD").unwrap(),
            })
            .await
            .unwrap();
        let user = service
            .create_user(
                ledger_id,
                super::NewUserName {
                    display_name: "Ada".to_owned(),
                },
            )
            .await
            .unwrap();
        let shares = vec![super::Share {
            user_id: user.user_id,
            shares: 1,
        }];
        let base_id = service
            .add_bill(
                ledger_id,
                super::NewBill {
                    amount_cents: 1000,
                    description: "Original".to_owned(),
                    payers: shares.clone(),
                    payees: shares.clone(),
                    prev: Vec::new(),
                },
            )
            .await
            .unwrap();
        let left_id = service
            .add_bill(
                ledger_id,
                super::NewBill {
                    amount_cents: 1200,
                    description: "Left branch".to_owned(),
                    payers: shares.clone(),
                    payees: shares.clone(),
                    prev: vec![base_id],
                },
            )
            .await
            .unwrap();
        let right_id = service
            .add_bill(
                ledger_id,
                super::NewBill {
                    amount_cents: 1500,
                    description: "Right branch".to_owned(),
                    payers: shares.clone(),
                    payees: shares,
                    prev: vec![base_id],
                },
            )
            .await
            .unwrap();
        (ledger_id, base_id, left_id, right_id)
    }
}
