# unbill-desktop

The React frontend served inside the Tauri webview. Responsible for all user-visible UI. Communicates with the Rust backend exclusively via Tauri's `invoke` and event listeners.

## Views

- **Ledger list** — shows all ledgers; the app's entry point.
- **Ledger view** — bills, users, and settlement summary for a selected ledger.
- **Add bill form** — payer, amount, description, and share-weight picker (equal or custom weights).
- **Settlement view** — who owes whom and the minimum set of transactions.

## Data flow

All data fetching goes through a single typed API module wrapping `invoke`. Events from the backend (ledger updated, peer connected) invalidate TanStack Query caches, triggering re-fetches. The frontend never holds stale data for long.

## Invariants

- The frontend never computes business logic — no settlement math, no amendment projection. It displays what the backend returns.
- All backend calls go through the typed API module. No raw `invoke` calls elsewhere.
- IDs are treated as opaque strings on the JavaScript side. The frontend does not parse or generate ULIDs.

## Failure modes

- Failed `invoke` calls surface as toast notifications.
- Stale data is handled by TanStack Query cache invalidation on service events.

## Open questions

- Routing: single-page with React state, or React Router? Decide at M5 based on actual screen count.
- Dark/light mode via Tailwind's `dark:` variants.
- i18n: deferred to post-M5.
