---
name: Unbill UI Native
desc: The Leptos frontend hosted by the Tauri desktop shell.
category:
  - concept
belongs:
  - frontend-ui
  - applications
refines:
  - ui-shared-model
---

`unbill-ui-native` is the Leptos frontend used by the Tauri desktop shell.
It implements the shared UI model with a dense native utility style.

The visual system favors full-height panes,
list rows,
simple tables,
compact toolbars,
restrained color,
system typography,
stable control dimensions,
and flush columns separated by single-pixel borders.
It avoids decorative backgrounds,
oversized cards,
nested cards,
preview mockups,
and ornamental gradients.
Safe-area insets are applied on the app shell so device notches and home indicators
do not require per-component overrides.

Navigation and utility controls use square icon-only buttons backed by Lucide SVG primitives.
Each has an accessible label,
hover title,
and hidden text equivalent.
Primary workflow actions remain text-first.

Mode is chosen from window width at startup and on resize.
Widths below 1200 px use compact mode.
Widths at or above 1200 px use ranger mode.
CSS media queries use the same cutoff.

Compact priority is detail, then bills, then ledgers.
Ranger columns are ledgers, bills, and detail or placeholder.
Settings fills the viewport in compact mode and overlays the three columns in ranger mode.
The status strip is hidden while idle,
shows the latest status or error message when present,
and shows a working chip while async operations are in flight.

`main.rs` mounts `App`.
`app.rs` owns navigation state and async bridge calls.
`pages.rs` defines screen components.
`components.rs` contains reusable app-level pieces.
`api.rs` mirrors JSON DTOs returned by Tauri commands.

The app stores bootstrap data,
selected ledger detail,
settings-only ledger detail,
create and join overlay state,
invite state,
saved-user import and share state,
conflict selections,
editor flows,
status,
errors,
and loading count in Leptos signals.

Mutating actions call the bridge,
show feedback,
refresh bootstrap state,
and refresh selected ledger detail only when the visible active ledger could have changed.

Device Settings shows the local device ID,
saved local users,
known peer devices,
saved-user share and import,
and ledger join actions.
Ledger Settings shows the ledger selector,
ledger users,
saved-user picker,
authorized peer devices,
and invitation actions.
Ledger-scoped sync actions refresh bootstrap state and selected ledger detail.

The native join action may prefill from the Tauri clipboard bridge.
Clipboard failures produce feedback,
but the editable join sheet remains available.

Bill editor draft behavior lives in `unbill-ui-components::bill_editor`.
The native app adapts DTOs into editor seeds and maps save requests to Tauri bridge inputs.

Conflict groups come from the Tauri bridge ledger detail DTO.
The UI renders them above settlement and sends selected bill plus competing set for resolution.
The backend validates the selection,
copies selected bill fields into a merge amendment,
and persists it through the service layer.

Timestamp DTOs stay as Unix milliseconds across the Tauri bridge.
The UI API layer formats them with the browser runtime's local calendar date and time.

Pure UI state helpers are unit-tested in the Leptos crate.
Tauri DTO assembly and command behavior are tested in the Tauri crate.

---

> **Sirno generated links begin. Do not edit this section.**

- belongs (to):
  - [applications](applications.md)
  - [frontend-ui](frontend-ui.md)
- belongs (from): (none)

> **Sirno generated links end.**
