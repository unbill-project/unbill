# service — Implementation

`inner.rs` contains `UnbillConsole`, `ServiceEvent`, and the helpers for device-local metadata. Opening the service is async: it primes a `Mutex<HashMap<LedgerId, LedgerDoc>>` cache by syncing every known ledger once, then starts an event bridge task that re-syncs the affected ledger whenever a `LedgerUpdated` event arrives from the channel.

Most public methods follow the same shape: take the target `LedgerDoc` from the cache (`take_doc`), apply one typed mutation or query, sync back to the device if the doc was mutated, then return the doc to the cache (`put_doc`). Read-only operations also use take/put to keep the doc out of the cache for the minimum time. Sync and invitation helpers reuse the same store and model vocabulary rather than creating a parallel transport-only model.
