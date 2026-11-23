# UnbillDevice runtime profiling

This suite runs **UnbillDevice directly**, with its real storage actor, CRDT
implementation, background tasks, and peer protocol. The runner has no dependency
on AsymChannel, Console, RPC, CLI, or UI. Transport-facing asym channel analysis is
reserved for a later extension.

## Run the suite

From the repository root, with the normal Rust toolchain and Python 3:

```sh
python3 tools/device-profile/suite.py --output target/profiles/first
```

This builds an optimized binary with debug symbols, then runs eight isolated
captures: SQLite and memory storage, each with idle, read, write, and auth
workloads. Open `target/profiles/first/index.html` to browse the reports. Every
capture also has `summary.json` and a Perfetto-compatible `trace.json`.

To include real device-to-device synchronization:

```sh
python3 tools/device-profile/suite.py \
  --output target/profiles/with-sync --include-sync
```

The sync workload needs working peer discovery/connectivity. It uses the normal
Iroh configuration, including mDNS, DNS publication and relay services. The first
session transfers a populated ledger delta; subsequent sessions measure unchanged
peers, under separate operation names. This is a same-host test, not a WAN or BLE
throughput benchmark. Network failures produce failed/partial reports, not zero
latencies or silently substituted in-memory networking.

On a headless machine, add `--headless` when building to disable BLE through the
existing test-helper feature. This **does not** disable IP networking or remote
relay/DNS activity, even in non-sync scenarios. The flag is stored in the capture
metadata; use ordinary builds to study the production BLE path.

For a single workload or a larger dataset:

```sh
cargo run --locked --profile profiling -p unbill-device-profile -- \
  --output target/profiles/read-large --store sqlite --scenario read \
  --ledgers 100 --bills 100 --iterations 100 --concurrency 8
python3 tools/device-profile/report.py target/profiles/read-large
```

`--concurrency` controls foreground tasks; `--worker-threads` controls Tokio
worker threads (default: the machine's available parallelism). Both are recorded
in metadata and checked when comparing runs.

The `--timeout-secs` default is 120, including fixture creation and startup. Raise
it for large datasets. Output directories must be fresh. The runner creates its
own databases and generated device identities there; it does not consult
`UNBILL_DATA_DIR`, `UNBILL_SOCKET`, existing accounts, or daemon processes. Treat
the generated database directories as disposable test data.

## What each workload measures

| Workload | Device path | Interpretation |
| --- | --- | --- |
| Startup, every run | Store open/migrations; device open; endpoint bind; registration/login | Fixture creation is separate from device startup; these are fresh device/store opens, not production cold-cache claims. |
| `idle` | No foreground operations for `--idle-secs` | Observe BLE snapshots, peer collection and SQLite notification polling while idle. |
| `read` | Authenticated account ledger listing and device peer collection | Each of `--concurrency` workers performs `--iterations` pairs. Peer collection loads CRDT docs and exposes growth with both ledger and bill counts. |
| `write` | Account ledger listing then metadata timestamp update | Concurrent device-local metadata writes; this is not a bill creation or CRDT write benchmark. |
| `auth` | Login, session lookup and logout | Sequential iterations expose password verification and session persistence separately from general reads. |
| `sync` | Two devices with shared, authorized fixture ledgers; initial delta then unchanged sync | Real device peer dial, protocol, CRDT and persistence work. `--iterations` controls unchanged sessions after the one initial session. |
| `all` | Idle, mixed reads/writes, auth, sync | A single mixed capture; the matrix suite uses separate scenarios for cleaner comparisons. |

Fixtures include synthetic bills and users. They are seeded through the raw
store **before** device ownership starts. All measured foreground work then goes
through the device or its authenticated account view.

## Read the timings

| Span | Meaning |
| --- | --- |
| `device.*`, `account.*` | Elapsed time of a direct service call, including downstream work and waiting. |
| `device.ble_poll` | Work in one snapshot poll; excludes the 500 ms timer wait. |
| `store.request [operation]` | Actor request from sending through execution completion. It continues if its caller cancels. |
| `store.backpressure [operation]` | Wait for a slot in the bounded actor queue. |
| `store.queue [operation]` | Wait after acquiring a queue slot until the actor dequeues the work. |
| `store.execute [operation]` | Work inside the serialized actor, including backend awaits. |
| `sqlite.blocking_queue` | Scheduling delay until the blocking closure begins. |
| `sqlite.mutex_wait` | Wait for the SQLite connection mutex. |
| `sqlite.execute` | Blocking backend work while holding the connection; includes transactions, serialization or merging done there, not only SQL engine time. |
| `sqlite.load_ledger`, `sqlite.save_ledger` | Complete backend operations, including CRDT work around the blocking closure. |
| `sqlite.notification_poll` | One notification query on its independent observer connection. |
| `crdt.*` | Decode, encode, merge, sync message generation/receipt and device-list hydration. |
| `peer.*` | Endpoint binding/dial, protocol exchange and completion barrier. |

The actor span carries parentage across the MPSC boundary. SQLite blocking work
also retains its parent. Each closed span has a unique capture ID and a
`parent` field in `spans.jsonl`; Perfetto exposes it as `parent_id` in span args.
A span closing describes its lifetime, including cancellation; it does not assert
operation success. The workload result records success/failure for the capture.
BLE and SQLite observer polls are independent background roots. The report
allowlists operation labels and timings rather than serializing tracing args.
Passwords, tokens, keys and ledger contents are excluded from the spans.

Durations are **inclusive**: a device call contains its actor request, which
contains SQLite work. Concurrent calls also overlap. Never sum these rows to
calculate total CPU use or a time breakdown. `entered_ms` means elapsed time
inside synchronous spans or instrumented async polls, including their children;
it is not a hardware CPU measurement. Lifetime-only markers such as
`store.request`, `store.queue`, `sqlite.blocking_queue` and `sqlite.mutex_wait`
are deliberately never entered and have zero entered time. Use their wall duration.

`runtime.jsonl` samples a 100 ms Tokio heartbeat. Linux also records process-wide
CPU ticks, resident memory and thread counts from `/proc`. CPU charts use the
recorded `CLK_TCK` and show percent of one core, so values may exceed 100%. On
other platforms those counters are unavailable; heartbeat and operation tracing
still work. RSS is a sampled peak, not an exact high-water mark or heap/allocation
profile. Resources cover setup as well as the workload. Heartbeat delays can
reflect scheduler load, blocking work or profiling overhead; they are not proof
of one cause.

## Compare changes and sampling overhead

Run the same suite configuration against two revisions on the same machine:

```sh
python3 tools/device-profile/suite.py --output target/profiles/before
# Switch/build the revision to investigate.
python3 tools/device-profile/suite.py \
  --output target/profiles/after --baseline target/profiles/before
```

Comparison rejects failed captures and mismatched workload/platform settings.
The HTML table and `summary.json` show p95 changes. Use repeated runs and enough
iterations before treating tail latency differences as regressions; results are
sensitive to CPU load, I/O caching, networking and instrumentation overhead.

The collector streams closed spans to disk synchronously, with bounded memory
for live spans. This makes captures recoverable but can perturb short operations
and storage timings. A `--no-trace` run uses identical workloads and resource
sampling with the span collector disabled. Compare whole-run elapsed/resource
data manually to estimate that overhead; span percentiles are unavailable in
this mode. Debug builds are marked in metadata and unsuitable for performance
conclusions. `profiling` inherits release optimization settings, uses thin LTO
and limited debug information to keep build memory manageable, and retains
symbols. It does not change the shipping release profile, which uses full LTO.
Set `CARGO_PROFILE_PROFILING_LTO=true` when exact full-LTO matching is needed and
the build machine has sufficient memory.

For actual CPU call stacks, sample the symbolized runner on the target OS. On
Linux with perf access:

```sh
cargo build --locked --profile profiling -p unbill-device-profile
perf record -F 99 --call-graph dwarf -o target/device-perf.data -- \
  target/profiling/unbill-device-profile --scenario read \
  --iterations 500 --no-trace --output target/profiles/cpu
perf report -i target/device-perf.data
```

On macOS the same binary can be selected in Instruments' Time Profiler.
Allocation stacks, Tokio task/waker diagnostics and asym channel boundaries are
future extensions; this suite makes no claim to collect them.

## Embed the collector or extend the suite

The `unbill-device-profile` library exports `Capture`, a tracing-subscriber layer.
An application embedding UnbillDevice may compose it with its existing subscriber
and a per-layer `unbill::profile` filter:

```rust
use tracing_subscriber::{filter::filter_fn, prelude::*};
use unbill_device_profile::{Capture, TARGET};

let capture = Capture::create(std::path::Path::new("device-spans.jsonl"))?;
tracing_subscriber::registry()
    .with(capture.clone().with_filter(filter_fn(|m| m.target() == TARGET)))
    .try_init()?;
// Open the store/device here, then run your application.
// Close the device and drain/shut down background tasks before finishing.
capture.finish()?;
```

Install the subscriber before opening the
store/device, retain a clone, close the device and call `Capture::finish()` to
surface write errors. Do not initialize a second global subscriber.

Only the standalone runner installs this layer today. Existing daemons/apps keep
their subscriber configuration. Later asym channel spans can become outer parents
of the same device spans without changing the device metrics or workloads.

## Validation

```sh
cargo test --locked -p unbill-device-profile --features headless
python3 -m unittest discover -s tools/device-profile -p 'test_*.py'
```

The runtime smoke tests exercise real memory and SQLite device calls, actor
parentage, blocking-thread parentage, payload exclusion and preservation of a
failed capture on timeout. The report tests check percentile definitions,
resource calculations, partial files and incompatible comparison rejection.
The optional manual workflow uploads reports for inspection without noisy
performance thresholds in normal CI.
