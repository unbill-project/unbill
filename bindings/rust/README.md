# Unbill Rust bindings

The `unbill` Cargo package (currently `0.0.10`) exposes the same `Console`
interface as the Python and Swift bindings. Calls go directly to the shared
Rust implementation. This crate does not depend on UniFFI; `../ffi` adapts this
crate for foreign languages.

Add the published crate from crates.io:

```toml
[dependencies]
unbill = "0.0.10"
```

For local development inside a checkout, use a path dependency instead:

```toml
[dependencies]
unbill = { path = "/path/to/unbill/bindings/rust" }
```

Open an embedded SQLite device, create a ledger and participants, then preview
and save a bill. Amounts are integer minor units (for USD, cents); share weights
are relative portions. Reuse the same bill ID for preview and save so rounding
is consistent:

```rust,no_run
use unbill::{Console, ConsoleBillDraft, ConsoleError, ConsoleShareInput, new_bill_id};

fn main() -> Result<(), ConsoleError> {
    let console = Console::open_in_process("./household-data".into())?;
    let ledger = console.create_ledger("Household".into(), "USD".into())?;
    let alice = console.create_user(ledger.ledger_id.clone(), "Alice".into())?;
    let bob = console.create_user(ledger.ledger_id.clone(), "Bob".into())?;

    let amount_cents = 4_200;
    let payers = vec![ConsoleShareInput { user_id: alice.user_id.clone(), shares: 1 }];
    let payees = vec![
        ConsoleShareInput { user_id: alice.user_id.clone(), shares: 1 },
        ConsoleShareInput { user_id: bob.user_id, shares: 1 },
    ];
    let bill_id = new_bill_id();
    let _preview = console.preview_bill_split(
        amount_cents, payers.clone(), payees.clone(), bill_id.clone(),
    )?;
    console.save_bill(
        ledger.ledger_id,
        amount_cents,
        "Dinner".into(),
        payers,
        payees,
        ConsoleBillDraft { bill_id, prev_bill_ids: vec![] },
    )?;
    Ok(())
}
```

This example uses `ConsoleBillDraft`, `ConsoleShareInput`, and `new_bill_id`;
`save_bill` returns the saved bill ID. For an amendment, put the IDs of the
superseded bills in `prev_bill_ids`.

On desktop, connect to an existing daemon with
`Console::open_rpc(unbill::default_data_directory()?)`. RPC honors `UNBILL_SOCKET`
and does not open the database. Android and iOS device and Simulator builds omit
this constructor; Mac Catalyst supports both constructors.

Both constructors return `Arc<Console>` and expose the same operations and record
types. IDs remain strings, amounts remain integer minor units, and errors use
`ConsoleError`. Helpers provide currency codes, bill IDs, build information, and
the embedded license catalog without opening a console.

## Runtime and callbacks

Each console owns its Tokio runtime. Synchronous methods, constructors, and the
final release of the console must run outside an async runtime context. In a
Tokio application, use `tokio::task::spawn_blocking` for those operations.
`sync_once` and `observe_ble` are async and can be awaited on the caller's executor.

Implement `ConsoleObserver` for service events and `ConsoleBleObserver` for BLE
snapshots. Callbacks run on Rust worker threads: enqueue work to your own thread
or event loop before calling synchronous console methods. Keep the returned
`ConsoleSubscription` alive, then call `cancel()` or drop it to unsubscribe.

An in-process console owns its data directory exclusively on desktop. Releasing
the last console handle stops its runtime and releases storage ownership. Daemon
consoles share the existing service and do not stop it when released.

## Verification

From the repository root:

```sh
cargo test -p unbill -p unbill-ffi
python3 bindings/rust/test_packaging.py
cargo tree -p unbill -e normal,build
```

The dependency tree must not contain UniFFI. The Rust tests cover persistence,
directory ownership, daemon connections, bill previews, and subscriptions. The
Python binding tests exercise the foreign adapter over this same implementation.

## Packaging and publishing

The package inherits the Cargo workspace version. With Cargo 1.96 or newer,
package and verify the Rust binding and its supporting crates together:

```sh
cargo fetch --locked
python3 bindings/rust/package.py
```

The helper checks for a clean checkout before generating packaging files.
Use `--allow-dirty` when checking uncommitted work. Packaging needs access to the
crates.io index. The command prepares a temporary license catalog
snapshot so the resulting archives build independently of the checkout.
Cargo verifies the archives before returning them under `target/package`.

After configuring crates.io access and checking the package names, publish the
same library set explicitly with `python3 bindings/rust/package.py --publish`.
Cargo orders the dependency uploads. The FFI adapter and applications are excluded.

### Automatic releases

`.github/workflows/rust.yml` verifies packages on `v*` tag pushes, on
`release/crates-v*` branches, through reusable workflow calls, and on manual
dispatch. It tests the binding and adapter, checks the dependency graph, and
verifies all package archives before a publishing job runs. A release ref must
match every selected library's version: for the current `0.0.10` workspace, use
`v0.0.10` (or `release/crates-v0.0.10`). Manual dispatch and reusable workflow
calls verify without publishing.

Set the `CARGO_REGISTRY_TOKEN` GitHub Actions secret on the repository or its
`crates-io` environment. The crates.io token needs publish access to `unbill` and
all selected `unbill-*` libraries, including permission to create new crates for
the first release. Confirm those names are available or owned by the publishing
account. Only the publishing step receives the token. Missing credentials fail
the release with a setup message.

After merging the workflow and configuring the secret, pushing a matching tag or
release branch to GitHub publishes the Rust libraries alongside the existing
Python release. Creating a local tag with `cargo release` does not upload to
crates.io by itself. To run the local version check for this workspace without
uploading, use `python3 bindings/rust/package.py --tag v0.0.10`. The helper
requires a clean checkout by default; add `--allow-dirty` only when intentionally
checking uncommitted changes.
