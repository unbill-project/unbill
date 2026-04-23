# unbill-tui

A keyboard-driven terminal frontend for unbill. Follows the shared UI model defined in `apps/DESIGN.md`. This document covers only what is specific to the terminal implementation.

## Terminal specifics

- Always renders in three-column layout. There is no compact mode.
- Input is keyboard-only; there is no mouse support.
- Rendered with `ratatui` on a `crossterm` backend.

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

| Key | Ledgers pane | Bills pane | Detail pane |
|-----|--------------|------------|-------------|
| `a` | Create ledger | Add bill | New bill |
| `e` | — | Amend selected bill | Amend bill |
| `d` | Delete ledger (confirmation required) | — | — |
| `u` | — | Ledger settings popup | — |
| `S` | Device settings popup | — | — |
| `q` | Quit | Quit | Quit |
| `Esc` | Close popup / cancel | Close popup / cancel | Close popup / cancel |

## Popup keyboard conventions

Navigation inside a popup follows the same `j`/`k` conventions as the main panes. `Tab` / `Shift+Tab` advance between form fields. `Enter` confirms. `Esc` closes without acting.

## Status bar

- **Left** — context-sensitive key hints for the focused pane. When a popup is open, shows `[Esc] close` instead.
- **Right** — sync status: idle (blank), syncing, or last error.
