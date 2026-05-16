---
name: Unbill UI Remote
desc: The browser frontend that connects to a hosted Unbill server over HTTP.
category:
  - concept
belongs:
  - frontend-ui
  - applications
refines:
  - ui-shared-model
---

`unbill-ui-remote` is a browser-only Leptos application compiled to WASM with Trunk.
It connects to a running `unbill-server` instance over HTTP.
All ledger state lives on the server-side device.

Startup resolves the server base URL from `UNBILL_SERVER_URL`
or the browser same-origin URL.
The app accepts an API key,
stores the key in browser local storage for later silent reconnect,
opens `HttpAsymChannel`,
and then opens `UnbillConsole` over that channel.

`main.rs` initializes the service and mounts the app after authentication.
`app.rs` owns navigation state.
`pages.rs` defines screen components.
`api.rs` wraps `UnbillConsole` calls and maps results to DTO structs.
Raw HTTP stays inside `HttpAsymChannel`;
the app layer calls service and API helpers rather than issuing ad hoc requests.

Remote UI data signals include device ID,
ledgers,
all users,
selected ledger detail,
and settings-only ledger detail.
Navigation and overlay signals include surface mode,
selected ledger ID,
settings popup state,
invitation URL,
full-screen sheet state,
and bill editor seed.
Feedback signals include status message,
error message,
and loading count.

After bootstrap,
one async task subscribes to service events.
On `LedgerUpdated`,
it refreshes the matching ledger summary,
all users,
selected ledger detail when no editor is open,
and settings ledger detail when the popup is viewing that ledger.
The task reads signals without creating reactive subscriptions.

Mutation handlers do not manually reload data.
They close relevant local UI state and set feedback,
then rely on the service event stream for projection refresh.

The bill editor is isolated from background detail refresh.
Its seed captures currency and users at open time,
and the page reads only the editor signal.

Device Settings shows the server-assigned device ID.
Remote UI does not expose peer sync controls there.
Ledger Settings provides ledger selection,
ledger users,
add-user picker,
and invitation generation.

Invitation URLs are written through the browser clipboard API.

The stylesheet follows the same dense pane system as the native UI:
system typography,
full-height panes,
compact toolbars,
restrained borders,
stable dimensions,
Lucide icon buttons,
text-first primary workflow actions,
and no padding or gaps around the ranger grid.

Trunk builds the static bundle.
`UNBILL_SERVER_URL` is set before build or serve when the server is not same-origin.
Pure DTO mapping and navigation helper tests live beside `api.rs` and `app.rs`.

---

> **Sirno generated links begin. Do not edit this section.**

- belongs (to):
  - [applications](applications.md)
  - [frontend-ui](frontend-ui.md)
- belongs (from): (none)

> **Sirno generated links end.**
