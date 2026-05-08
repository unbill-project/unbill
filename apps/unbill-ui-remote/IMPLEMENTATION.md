# unbill-ui-remote Implementation

The app is a client-side rendered Leptos application compiled to WASM with Trunk. `main.rs` initializes the `UnbillService` and mounts `App`. `app.rs` owns navigation state and calls into `api.rs`. `pages.rs` defines screen-level components. `api.rs` wraps `UnbillService` calls and maps results to plain DTO structs.

## Service initialization

`main.rs` reads `UNBILL_SERVER_URL` (baked in at compile time via `env!`), constructs an `HttpStore` with that base URL, and calls `UnbillService::open(store)`. The resulting service is passed into `App` as a prop and stored in a context so all descendant components can reach it.

## State model

App-level state in `app.rs` is split into granular `RwSignal`s so that each piece of UI only re-renders when its own data changes.

**Data signals** (populated from the server):

| Signal | Type | Meaning |
|---|---|---|
| `device_id` | `String` | Server-assigned device ID. Read-only display. |
| `ledgers` | `Vec<LedgerSummary>` | All ledgers, sorted by most-recent bill then name. |
| `all_users` | `Vec<User>` | Union of users across all ledgers. Used by the add-user picker. |
| `ledger_detail` | `Option<LedgerDetail>` | Bills, users, devices, and settlement for the currently selected ledger. |
| `settings_ledger_detail` | `Option<LedgerDetail>` | Same shape, but for the ledger selected inside the Settings popup — independent so Settings can browse ledgers without changing the main view. |

**Navigation / overlay signals** (owned entirely by the UI layer):

| Signal | Type | Meaning |
|---|---|---|
| `surface_mode` | `SurfaceMode` | Compact vs. Ranger, updated from the window resize listener. |
| `selected_ledger_id` | `Option<String>` | Which ledger is open in the main column. |
| `settings_popup` | `Option<SettingsPopupState>` | Whether the Settings popup is open, which tab is active, and which ledger is selected inside it. |
| `invitation_url` | `Option<String>` | Last generated invitation URL, cleared when the popup closes. |
| `overlay` | `Option<OverlayKind>` | Which full-screen sheet is open (`CreateLedger` or `AddUser`). |
| `bill_editor` | `Option<BillEditorSeed>` | The form state for the bill editor. `None` means the editor is closed. |

**Feedback signals**:

| Signal | Type | Meaning |
|---|---|---|
| `status_message` | `Option<String>` | Last success message shown in the status strip. |
| `error_message` | `Option<String>` | Last error message shown in the status strip. |
| `loading_count` | `usize` | Number of in-flight async operations. Incremented before each `spawn_local`, decremented (saturating) in its completion handler. `loading_count != 0` drives the busy indicator. |

## Service event loop

After the initial bootstrap load, a single `spawn_local` subscribes to `ServiceEvent` via `api::subscribe()`. On `LedgerUpdated { ledger_id }` it:

1. Refreshes the matching entry in `ledgers` (in-place update, preserves order via re-sort).
1. Refreshes `all_users`.
1. If `selected_ledger_id == ledger_id` and the bill editor is not open, refreshes `ledger_detail`.
1. If the Settings popup has `selected_ledger_id == ledger_id`, refreshes `settings_ledger_detail`.

All reads from signals inside the event loop use `get_untracked()` to avoid creating reactive subscriptions on the non-reactive async task.

`ServiceEvent::LedgerUpdated` is emitted by `UnbillService` on both local mutations (`add_bill`, `add_user`, `create_user`) and incoming network sync. Mutation handlers in `app.rs` therefore do not call reload functions themselves — they only set `bill_editor.set(None)` and the status/error messages.

## Bill editor isolation

`BillEditorSeed` carries a snapshot of `currency` and `users` taken at the moment the editor is opened. `BillEditorPage` reads only `bill_editor`, not `ledger_detail`, so background refreshes to `ledger_detail` do not destroy or reset the open form.

## API layer

`api.rs` defines async functions that delegate to `UnbillService` and return typed DTO structs. DTOs are plain `serde` structs defined alongside the functions. `main.rs` and `app.rs` call into `api.rs`; no component accesses the service directly.

Timestamp DTOs stay as Unix milliseconds from service state through the API layer. Display formatting happens in `api.rs` by reading the viewer's local calendar date and time from the browser runtime and rendering a zero-padded year/month/day hour:minute value.

## Navigation and settings

Settings state is a single popup state value holding the active tab and selected ledger ID. In ranger mode the popup overlays three columns. In compact mode it fills the viewport. Responsive mode selection uses the same 1200 px breakpoint and window resize listener as the shared UI model.

Device Settings renders the server-assigned device ID (read-only). Ledger Settings renders a ledger selector, ledger users, an add-user picker, and invitation URL generation. There are no sync controls.

## Clipboard

Invitation URLs are written via the browser `navigator.clipboard.writeText` API through a `web-sys` binding.

## Stylesheet

System typography, full-height panes, dense rows, compact toolbars, restrained borders, stable control dimensions. Navigation and utility controls use Lucide SVG icon buttons. Primary workflow actions use text buttons.

## Dependencies

- `leptos` (csr feature)
- `unbill-core` (no `network` feature)
- `unbill-store-http`
- `wasm-bindgen`, `wasm-bindgen-futures`
- `web-sys` (clipboard, window, event)
- `unbill-ui-components`
- `serde`, `serde_json`
- `console_error_panic_hook`

## Build

`trunk build` produces a static bundle in `dist/`. `UNBILL_SERVER_URL` must be set in the environment before building. `trunk serve` rebuilds on change and proxies to the same origin by default.

## Tests

Pure state helpers (DTO mapping, navigation state transitions) are unit-tested in `#[cfg(test)]` modules in `api.rs` and `app.rs`.
