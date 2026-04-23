use serde::{Deserialize, Serialize, de::DeserializeOwned};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(module = "/src/bridge.js")]
extern "C" {
    #[wasm_bindgen(js_name = hasTauriRuntime)]
    fn has_tauri_runtime_js() -> bool;

    #[wasm_bindgen(catch, js_name = invokeJson)]
    async fn invoke_json_js(command: &str, args_json: &str) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(catch, js_name = readClipboardText)]
    async fn read_clipboard_text_js() -> Result<JsValue, JsValue>;

    #[wasm_bindgen(catch, js_name = writeClipboardText)]
    async fn write_clipboard_text_js(text: &str) -> Result<JsValue, JsValue>;
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppBootstrap {
    pub device_id: String,
    pub ledgers: Vec<LedgerSummary>,
    pub local_users: Vec<LocalUser>,
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
pub struct LocalUser {
    pub user_id: String,
    pub display_name: String,
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
pub struct AddLocalUserInput {
    pub display_name: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddUserInput {
    pub ledger_id: String,
    pub display_name: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JoinLedgerInput {
    pub url: String,
    pub label: String,
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

pub async fn bootstrap_app() -> Result<AppBootstrap, String> {
    invoke("bootstrap_app", &()).await
}

pub async fn create_ledger(input: CreateLedgerInput) -> Result<LedgerSummary, String> {
    invoke("create_ledger", &serde_json::json!({ "input": input })).await
}

pub async fn load_ledger_detail(ledger_id: &str) -> Result<LedgerDetail, String> {
    invoke(
        "load_ledger_detail",
        &serde_json::json!({ "ledgerId": ledger_id }),
    )
    .await
}

pub async fn add_local_user(input: AddLocalUserInput) -> Result<LocalUser, String> {
    invoke("add_local_user", &serde_json::json!({ "input": input })).await
}

pub async fn add_user(input: AddUserInput) -> Result<User, String> {
    invoke("add_user", &serde_json::json!({ "input": input })).await
}

pub async fn create_invitation(ledger_id: &str) -> Result<String, String> {
    invoke(
        "create_invitation",
        &serde_json::json!({ "ledgerId": ledger_id }),
    )
    .await
}

pub async fn join_ledger(input: JoinLedgerInput) -> Result<(), String> {
    invoke("join_ledger", &serde_json::json!({ "input": input })).await
}

pub async fn save_bill(input: SaveBillInput) -> Result<String, String> {
    invoke("save_bill", &serde_json::json!({ "input": input })).await
}

pub async fn sync_once(peer_node_id: &str) -> Result<(), String> {
    invoke(
        "sync_once",
        &serde_json::json!({ "peerNodeId": peer_node_id }),
    )
    .await
}

pub async fn read_clipboard_text() -> Result<String, String> {
    let value = read_clipboard_text_js().await.map_err(js_error_to_string)?;
    js_value_to_string(value)
}

pub async fn write_clipboard_text(text: &str) -> Result<(), String> {
    write_clipboard_text_js(text)
        .await
        .map(|_| ())
        .map_err(js_error_to_string)
}

async fn invoke<T, A>(command: &str, args: &A) -> Result<T, String>
where
    T: DeserializeOwned,
    A: Serialize,
{
    let args_json = serde_json::to_string(args).map_err(|error| error.to_string())?;
    let value = invoke_json_js(command, &args_json)
        .await
        .map_err(js_error_to_string)?;
    let payload = js_value_to_string(value)?;
    serde_json::from_str(&payload).map_err(|error| error.to_string())
}

fn js_value_to_string(value: JsValue) -> Result<String, String> {
    value
        .as_string()
        .ok_or_else(|| "unexpected non-string JavaScript result".to_owned())
}

fn js_error_to_string(value: JsValue) -> String {
    value
        .as_string()
        .or_else(|| {
            js_sys::JSON::stringify(&value)
                .ok()
                .and_then(|value| value.as_string())
        })
        .unwrap_or_else(|| "unknown JavaScript error".to_owned())
}

pub fn format_money(amount_cents: i64, currency: &str) -> String {
    let sign = if amount_cents < 0 { "-" } else { "" };
    let absolute = amount_cents.abs();
    let units = absolute / 100;
    let cents = absolute % 100;
    format!("{sign}{currency} {units}.{cents:02}")
}

pub fn format_timestamp(timestamp_ms: i64) -> String {
    let seconds = timestamp_ms / 1000;
    let day = seconds / 86_400;
    format!("day {day}")
}
