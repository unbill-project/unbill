# unbill-tui

A full-featured interactive terminal frontend for unbill. It presents the same capabilities as the CLI in a persistent, keyboard-driven interface without requiring a graphical desktop environment.

## Layout

The screen is divided into three vertical panes that are always visible.

```
┌────────────────┬───────────────────────┬──────────────────────────┐
│ Ledgers        │ Bills                 │ Detail                   │
│                │                       │                          │
│ > Household    │ > Dinner      $30.00  │                          │
│   Trip         │   Lunch       $12.50  │                          │
│   Road trip    │   Coffee       $5.00  │                          │
│                │                       │                          │
│                │                       │                          │
├────────────────┴───────────────────────┴──────────────────────────┤
│ [j/k] move  [h/l] pane  [a]dd  [e]dit  [u]sers  [s]ettle  [q]uit │
└───────────────────────────────────────────────────────────────────┘
```

- **Left pane — Ledger list.** All ledgers on this device. Moving the cursor through this list immediately updates the bill pane to show bills for the cursor-focused ledger.
- **Middle pane — Bill list.** Effective bills in the cursor-focused ledger, shown with description and amount.
- **Right pane — Detail.** Reserved for a future iteration; currently empty.

## Focus and Navigation

One pane is active at a time. The active pane has a highlighted border.

| Key | Action |
|-----|--------|
| `h` | Move focus to left pane |
| `l` | Move focus to right pane |
| `Tab` | Move focus right (wraps) |
| `Shift+Tab` | Move focus left (wraps) |
| `Enter` | Move focus right (same as `l`) |
| `j` / `k` | Move cursor down / up within the focused pane |
| `g` | Jump to first item |
| `G` | Jump to last item |

## Actions

Actions are context-sensitive. The status bar shows only the keys valid for the currently focused pane.

| Key | Ledger pane | Bill pane |
|-----|-------------|-----------|
| `a` | Create ledger | Add bill |
| `e` | — | Amend selected bill |
| `d` | Delete ledger (confirmation required) | — |
| `u` | Manage users in ledger | — |
| `s` | Open settlement | — |
| `S` | Open device settings | — |
| `i` | Open invite / join | — |
| `q` | Quit | Quit |
| `Esc` | Close popup / cancel | Close popup / cancel |

Bills are append-only and cannot be deleted.

## Status Bar

The status bar at the bottom of the screen has two parts:

- **Left** — context-sensitive key hints for the currently focused pane. Updates immediately when focus changes.
- **Right** — sync status indicator: idle, syncing, or last error.

## Popup Window

Every action that requires input or displays a secondary view opens a popup window centered over the main panes. The background dims. Only one popup is visible at a time. Multi-step flows open a second popup sequentially: the first popup closes and the second opens in its place.

Navigation inside a popup follows the same `j`/`k` conventions as the main panes. `Tab` / `Shift+Tab` advance between form fields. `Enter` confirms. `Esc` closes the popup without acting.

### Create ledger
Form with two fields: name and ISO 4217 currency code.

### Add bill
Form with fields: description, amount (decimal), payer (selected from ledger users with `j`/`k`), share users (multi-select from ledger users).

### Amend bill
Same form as add bill, pre-filled from the selected bill. The `prev` link to the selected bill is set automatically on confirm.

### User management
Lists the current users in the focused ledger. An add action opens a sub-popup (sequentially) showing the device's saved local users to choose from. A create action in that same sub-popup creates a new local user and adds them.

### Settlement — pick user
Lists the saved local users on this device. Selecting one and pressing `Enter` closes this popup and opens the settlement result popup.

### Settlement — result
Shows the net transactions for the chosen user across all ledgers. Read-only; `Esc` closes.

### Device settings
Shows the device ID and known peer labels. An action allows entering a peer `NodeId` to trigger a manual sync.

### Invite / join
Two tabs navigated with `h`/`l`:
- **Invite** — generates and displays an `unbill://join/...` URL for the focused ledger.
- **Join** — a text field to paste an inbound `unbill://join/...` URL and join the ledger.

### Confirm delete
A yes/no prompt shown before deleting a ledger. `Enter` on yes confirms; `Esc` or `Enter` on no cancels.

## Sync

A background task subscribes to `ServiceEvent`s from the service and refreshes the visible panes when a `LedgerUpdated` event arrives. Manual sync is initiated from the device settings popup.

## Empty States

- No ledgers: left pane shows a dim `no ledgers — press [a] to create one`.
- No bills: middle pane shows a dim `no bills — press [a] to add one`.
- No local users (for settlement): settlement pick-user popup shows `no saved users — create one first`.

## Principles

- No business logic lives in the TUI. All mutations go through `UnbillService`.
- The TUI treats the service as the source of truth and re-reads it after every mutation.
- The bill pane always reflects the ledger under the cursor, not the last confirmed selection. Navigation is immediate.
- Keyboard shortcuts follow vim conventions where there is a natural mapping; other keys are mnemonic.
- Errors from the service are shown in the status bar and never crash the TUI.
