# Unbill UI Leptos Implementation

The Leptos app is a client-side rendered Tauri frontend. `main.rs` mounts `App`, `app.rs` owns navigation state and async bridge calls, `pages.rs` defines screen-level components, `components.rs` contains reusable UI pieces, and `api.rs` mirrors the JSON DTOs returned by Tauri commands.

The app keeps backend data in signals: bootstrap state for the device ID, ledgers, local users, and known devices; selected ledger detail for bills, users, conflict groups, settlement, and ledger-scoped sync peers; settings ledger detail for overlay-only ledger selection; and transient overlay state for create, join, invite, saved-user import/share, conflict selection, and editor flows. Mutating actions call the bridge, show shared status or error feedback, refresh bootstrap state, and refresh selected ledger detail only when the visible active ledger could have changed.

Settings state is represented as a single popup state with an active tab and selected ledger ID. In ranger mode the popup overlays the three columns. In compact mode the popup fills the viewport while the normal compact page priority remains unchanged behind it.

Responsive mode selection uses a pure width helper: widths below 1200 px render compact mode, and widths at or above 1200 px render ranger mode. `App` stores the current mode in a signal and updates it from a window resize listener. CSS media queries use the same cutoff so overlay and pane styling switch with the Rust render mode.

Device Settings renders the device ID, saved local users, known peer devices across local ledgers, saved-user share/import actions, and ledger join actions. Ledger Settings renders a ledger selector, ledger users, saved-user picker for adding users to the selected ledger, authorized peer devices, and ledger invitation actions. Each peer row triggers the shared one-shot sync bridge command.

The native join action attempts to prefill the join sheet from the platform clipboard. Empty clipboard text or clipboard read failures are surfaced through the shared toast feedback, but the join sheet still opens with an editable invitation URL field so users can paste manually.

The bill editor page, draft model, split preview, and save-time validation live in `unbill-ui-components::bill_editor`. This app adapts native DTOs into the shared editor seed and maps the returned save request into the Tauri bridge input.

Conflict groups are included in the ledger detail DTO assembled by the Tauri bridge. The native UI renders them above settlement, stores the selected bill per group in Leptos state, and sends a conflict resolution command that names the selected bill and the full competing bill set. The Tauri backend validates the selection against the detected group, copies the selected bill fields into a merge amendment, and persists it through the service layer.

The stylesheet implements a native utility shell with system typography, full-height panes, dense rows, compact toolbars, restrained borders, and stable control dimensions. Screen frame columns sit flush against each other separated only by a single-pixel right border on each frame (omitted on the last child). The topbar uses a flex row with the copy slot growing to fill available space; leading and trailing slots do not shrink. Safe area insets are applied as padding on `.app-shell` using `env(safe-area-inset-*)` so device notches and home indicators are respected without per-component overrides.

Navigation and utility controls render through a shared icon-button component backed by static Lucide SVG primitives. The component maps app actions to icons, visible hover titles, accessible labels, and hidden text. It is used for topbar navigation, settings and sheet close buttons, per-row sync/share/copy/add utilities, and the bill editor save control. Larger primary workflow actions continue to use text buttons.

Timestamp DTOs stay as Unix milliseconds across the Tauri bridge. The UI API layer formats them for display by asking the browser runtime for the viewer's local calendar date and time, then rendering a zero-padded year/month/day hour:minute value.

Tests live beside the Rust code they cover. Pure UI state helpers are unit-tested in the Leptos crate, while Tauri bridge DTO assembly and command behavior are tested in the Tauri crate.
