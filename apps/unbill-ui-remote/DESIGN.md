# unbill-ui-remote

Browser-only web frontend that connects to a running `unbill-server` instance over HTTP. It implements the shared UI model from `apps/DESIGN.md`. The app compiles to WASM and is served as a static bundle built with Trunk.

## Purpose

`unbill-ui-remote` is for users who access unbill through a browser pointed at a hosted server. All ledger state lives on the server. The browser has no local storage of its own.

## API layer

At startup the app creates an `HttpStore` pointing at the server base URL, then opens an `UnbillConsole` over it. `src/api.rs` wraps `UnbillConsole` calls and maps results to plain DTO structs consumed by the component layer. All HTTP communication is handled inside `unbill-store-http`; the app layer makes no raw HTTP requests.

The server base URL is baked in at build time via the `UNBILL_SERVER_URL` environment variable, defaulting to an empty string (same-origin).

DTOs are plain `serde` structs defined in `api.rs`. They are shared with the component layer through `Callback` and `Signal` props.

## Component layer

All UI components come from `crates/unbill-ui-components`. The app provides no components of its own. App-level wiring — signals, callbacks, async tasks — lives in `src/app.rs` and `src/pages.rs`.

## Device identity

The server manages device identity. Device Settings shows the server-assigned device ID (read-only) and no sync controls.

## Clipboard

Invitation URLs are copied through the browser `navigator.clipboard` API.

## Screen frame header

Each column's topbar header is supplied by the caller as an optional `AnyView` fragment, following the same convention as the native app. The fragment contains leading, copy, and trailing flex children.

## Settings popup

The settings popup title bar shows only the "Settings" heading. The device ID appears inside the Device Settings tab body.

## Ledger list

The ledgers page renders the ledger rows directly inside the screen content area with no surrounding section card.

## Visual system

Dense panes, list rows, simple tables, compact toolbars, system typography, Lucide icons. Screen frame columns are flush against each other with no gap or padding around the grid.

## Build tooling

Built with Trunk. `Trunk.toml` points at `index.html`. `UNBILL_SERVER_URL` is set as an environment variable before running `trunk build` or `trunk serve`.
