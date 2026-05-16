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
    ├── settings.rs      — SettingsPopup: Device Settings tab + Ledger Settings tab
    ├── invite.rs        — InviteResultPopup only
    └── confirm.rs
```

## Dependencies

- `ratatui` — terminal rendering
- `crossterm` — terminal backend and raw-mode input
- `tokio` — async runtime (shared with unbill-core)
- `unbill-core` — all domain logic via `UnbillConsole`

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
1. **Service events** — `broadcast::Receiver<ServiceEvent>` from `UnbillConsole::subscribe()`. A `LedgerUpdated` event refreshes bills, users, and settlement.
1. **Render tick** — a 16 ms interval (~60 fps) that triggers a redraw unconditionally.

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

`PopupAction` carries the data for the service call. The event loop matches on the action and calls the appropriate `UnbillConsole` method.

## Settings popup

`SettingsPopup` (in `popup/settings.rs`) is the single settings overlay. It has two top-level tabs selectable with `Tab` / `Shift+Tab`:

- **Device Settings** — device ID display, saved-user list (add, import, share, peer sync). Field navigation uses `Tab` / `Shift+Tab`; `Enter` confirms the focused field.
- **Ledger Settings** — a ledger selector at the top (j/k to move, Enter or Tab to move focus to content) and per-ledger content below (Users sub-tab: ledger user list + add-from-device; Invite sub-tab: generate invite URL). `h`/`l` switch sub-tabs within the content area.

`app.rs` constructs `SettingsPopup` via `open_settings_popup`, which pre-loads device ID, saved local users, all ledger metadata, and ledger users for each ledger before opening the popup. The initial tab (`TopTab::Device` or `TopTab::Ledger`) and the pre-selected ledger cursor are passed at construction time.

Generating an invite returns `Action(GenerateInvite { ledger_id })`, which the app handles by calling `svc.create_invitation` and then opening `InviteResultPopup` via direct assignment to `state.popup`.

## Testing

The TUI has no unit tests of its own. Domain correctness is covered by `unbill-core` tests. The TUI is validated manually against the same `UnbillConsole` that the CLI e2e tests exercise.
