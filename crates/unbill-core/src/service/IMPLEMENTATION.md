# service — Implementation

`inner.rs` contains `UnbillService`, `ServiceEvent`, `LocalUser`, and the helpers for device-local metadata. Opening the service loads or creates the device key and initializes the event broadcaster.

Most public methods follow the same shape: load the target ledger document, apply one typed mutation or query, save updated bytes and metadata if needed, then return domain results. Sync and invitation helpers reuse the same store and model vocabulary rather than creating a parallel transport-only model.
