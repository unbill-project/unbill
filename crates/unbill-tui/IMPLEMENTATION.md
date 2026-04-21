# unbill-tui — Implementation

## Crate structure

```
src/
├── main.rs         — entry point: open service, run event loop
├── app.rs          — AppState and top-level event dispatch
├── ui.rs           — top-level render function; composes panes, popup, status bar
├── pane/
│   ├── mod.rs      — Pane enum (Ledger, Bills, Detail) and shared types
│   ├── ledger.rs   — render and key handling for the ledger list pane
│   └── bills.rs    — render and key handling for the bill list pane
└── popup/
    ├── mod.rs           — PopupView trait, PopupOutcome, active popup dispatch
    ├── create_ledger.rs
    ├── add_bill.rs
    ├── amend_bill.rs
    ├── users.rs
    ├── settlement.rs    — pick-user and result views, each a separate PopupView
    ├── device.rs
    ├── invite.rs
    └── confirm.rs
```

## Dependencies

- `ratatui` — terminal rendering
- `crossterm` — terminal backend and raw-mode input
- `tokio` — async runtime (shared with unbill-core)
- `unbill-core` — all domain logic via `UnbillService`

## AppState

`AppState` is the single source of UI state. It holds no ledger data directly — domain data is fetched from the service and held only for the duration of a render frame.

```
focused_pane: Pane
ledger_cursor: usize
bill_cursor: usize
popup: Option<Box<dyn PopupView>>
sync_status: SyncStatus        // Idle | Syncing | Error(String)
status_message: Option<String> // transient error or info shown in status bar
```

After every mutation the relevant cursor is bounds-checked and the cached data is invalidated so the next render re-fetches from the service.

## Event loop

The main loop runs in a single tokio task and selects across three concurrent streams:

1. **Terminal events** — crossterm key and resize events via `EventStream`.
2. **Service events** — `broadcast::Receiver<ServiceEvent>` from `UnbillService::subscribe()`. A `LedgerUpdated` event clears the bill cache and schedules a redraw.
3. **Render tick** — a 16 ms interval (~60 fps) that triggers a redraw unconditionally.

Key events are routed first to the active popup (if any), then to the focused pane.

## Rendering

`ui.rs` calls `Layout::horizontal` to divide the terminal into three columns (roughly 20 % / 40 % / 40 %) and a fixed one-line status bar at the bottom. Each pane module exposes a `render(frame, area, state, data)` function.

When a popup is active, `ui.rs` renders the main layout first (dimmed), then draws the popup centered over the screen using `ratatui`'s `Clear` widget followed by a `Block`-framed area.

Focused pane borders are styled bright; unfocused borders are dim.

## PopupView trait

`PopupView` is a trait with two methods:

```rust
fn render(&self, frame: &mut Frame, area: Rect);
fn handle_key(&mut self, key: KeyEvent) -> PopupOutcome;
```

`PopupOutcome` is an enum:

```rust
enum PopupOutcome {
    Pending,                    // popup stays open, no action
    Cancelled,                  // close popup, no action
    Action(PopupAction),        // close popup, call service with this action
    OpenNext(Box<dyn PopupView>), // replace current popup with a new one
}
```

`PopupAction` carries the data for the service call (e.g. `CreateLedger { name, currency }`, `AddBill { ... }`, `DeleteLedger { id }`). The event loop matches on the action and calls the appropriate `UnbillService` method.

The `OpenNext` variant handles sequential multi-step flows: the settlement pick-user popup returns `OpenNext(Box::new(SettlementResultPopup { user_id }))` when the user confirms their selection.

## Status bar hints

Each `Pane` variant has a `hints()` method returning a slice of `(key, label)` pairs. The status bar renders them as `[key] label` and rebuilds the string only when focus changes. When a popup is open the hints area shows `[Esc] close` instead.

## Testing

The TUI has no unit tests of its own. Domain correctness is covered by `unbill-core` tests. The TUI is validated manually against the same `UnbillService` that the CLI e2e tests exercise.
