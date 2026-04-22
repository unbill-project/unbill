# unbill-tui — Implementation

## Crate structure

```
src/
├── main.rs          — entry point
├── app.rs           — AppState, event loop, key routing
├── ui.rs            — top-level render
├── pane/
│   ├── mod.rs       — Pane enum and hints
│   ├── ledger.rs    — ledger list render
│   ├── bills.rs     — bill list + inline settlement render
│   └── detail.rs    — bill detail view + bill editor (BillEditor, ParticipantRow, EditorSection)
└── popup/
    ├── mod.rs           — PopupView trait, PopupOutcome, PopupAction
    ├── create_ledger.rs
    ├── ledger_settings.rs — users tab + invite tab (was users.rs)
    ├── device.rs
    ├── invite.rs        — InviteResultPopup only
    └── confirm.rs
```

## Dependencies

- `ratatui` — terminal rendering
- `crossterm` — terminal backend and raw-mode input
- `tokio` — async runtime (shared with unbill-core)
- `unbill-core` — all domain logic via `UnbillService`

## AppState

```
focused_pane: Pane
ledger_cursor: usize
bill_cursor: usize
ledgers: Vec<LedgerMeta>
users: Vec<User>             — ledger users for current ledger
bills: Vec<Bill>
settlement: Vec<SettlementTransaction>
bill_editor: Option<BillEditor>
popup: Option<Box<dyn PopupView>>
sync_status: SyncStatus
status_message: Option<String>
```

## Event loop

The main loop runs in a single tokio task and selects across three concurrent streams:

1. **Terminal events** — crossterm key and resize events via `EventStream`.
2. **Service events** — `broadcast::Receiver<ServiceEvent>` from `UnbillService::subscribe()`. A `LedgerUpdated` event refreshes bills, users, and settlement.
3. **Render tick** — a 16 ms interval (~60 fps) that triggers a redraw unconditionally.

Key events are routed first to the active popup (if any), then to the bill editor (if active and Detail pane is focused), then to the focused pane.

## Rendering

`ui.rs` calls `Layout::horizontal` to divide the terminal into three columns (roughly 20 % / 40 % / 40 %) and a fixed one-line status bar at the bottom. Each pane module exposes a `render(frame, area, state)` function.

When a popup is active, `ui.rs` renders the main layout first, then draws the popup centered over the screen using `ratatui`'s `Clear` widget followed by a `Block`-framed area.

Focused pane borders are styled bright; unfocused borders are dim.

## Bill editor

`BillEditor` lives in `pane/detail.rs` and is stored in `AppState.bill_editor`. It covers both "new bill" and "amend bill" workflows. When `bill_editor` is `Some`, the Detail pane renders the editor form; when `None` it renders a read-only bill detail view.

The editor has four sections (Description, Amount, Payers, Payees). Tab / Enter advance through sections; Esc closes without saving. Confirming on the Payees section validates and calls `svc.add_bill`.

## Settlement inline rendering

After the bill list, `pane/bills.rs` renders a separator and per-ledger settlement transactions resolved against `AppState.users`. `refresh_settlement` calls `svc.settle_ledger` and stores results in `AppState.settlement`.

## PopupView trait

`PopupView` is a trait with two methods:

```rust
fn render(&self, frame: &mut Frame, area: Rect);
fn handle_key(&mut self, key: KeyEvent) -> PopupOutcome;
```

`PopupOutcome` is an enum:

```rust
enum PopupOutcome {
    Pending,
    Cancelled,
    Action(PopupAction),
    OpenNext(Box<dyn PopupView>),
}
```

`PopupAction` carries the data for the service call. The event loop matches on the action and calls the appropriate `UnbillService` method.

## Ledger settings popup

`LedgerSettingsPopup` (in `popup/ledger_settings.rs`) replaces the old `UsersPopup` and `InvitePopup`. It has two tabs: Users (read-only list + add-from-local) and Invite (press Enter to generate URL). Generating an invite returns `Action(GenerateInvite { ledger_id })`, which the app handles by calling `svc.create_invitation` and then opening `InviteResultPopup` via direct assignment to `state.popup`.

## Testing

The TUI has no unit tests of its own. Domain correctness is covered by `unbill-core` tests. The TUI is validated manually against the same `UnbillService` that the CLI e2e tests exercise.
