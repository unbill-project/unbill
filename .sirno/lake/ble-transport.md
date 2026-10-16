---
core.name: BLE Transport
core.desc: Default-enabled BLE discovery and Iroh transport with observable peer snapshots.
core.category:
  - core.concept
core.belongs:
  - symmetric-channel
---

Apple application bundles declare Bluetooth usage. Standalone macOS daemon and
server binaries embed an Info.plist with a Bluetooth usage description. The CLI
exposes `device ble` (including JSON output) through the connected daemon.

BLE starts automatically as an additional path for the existing Iroh identity and sync/join protocols. IP transports, mDNS, relays, and ledger authorization remain enabled. There is no runtime switch or public configuration to disable BLE. Initialization failure is observable and leaves IP connectivity available. Upstream supports macOS, iOS, Linux, and Android; Windows reports unavailable. Android also requires its host BLE plugin and permissions. The internal test-helpers build feature bypasses radio initialization so automated fixtures never request Bluetooth permission.

BLE actors run on an owned runtime thread so endpoint closure or drop cancels the upstream background tasks and releases radio ownership. Initialization is not cancelled midway: the upstream builder bounds its steps internally. A snapshot reports disabled, active, unavailable, or closed state and transient peer phases. Advertisements alone are not authenticated NodeIds or proof of current proximity. Only verified_endpoint is mapped to a NodeId, and discovery never authorizes ledger access.

The device-to-console boundary exposes a read-only snapshot through local, RPC, and authenticated HTTP channels, and UnbillConsole delegates it. Peer snapshots are process-local and are not persisted in ledger documents. Physical BLE exchange requires two separate machines; unit tests use synthetic snapshots and preserve IP transport regression coverage.

The device samples its in-memory BLE snapshot every 500 ms using one weakly owned task per device, publishing only changed values through a watch channel. Local, RPC and authenticated SSE forward BLE change notifications. Console subscribe_ble returns a watch receiver initialized with the current snapshot, then refreshes on notifications (including lag recovery). Dropping the receiver stops its console task; dropping or closing the device stops radio polling. Snapshot reads remain available for one-shot consumers.

BleSnapshot::connected_node_ids returns unique verified identities only while the transport is Active and the peer phase is Connected. Frontends use this projection for connection badges; it does not claim the active sync traffic uses BLE.

Discovery indicators represent only peers present in the current transport snapshot, the same list shown by the CLI. Known ledger NodeIds are correlated with current peer device IDs through device_for_endpoint, carrying matches in discovered_node_ids separately from verified identities. Historical scan hints alone cannot produce an indicator. Removing a peer removes its indicator on the next sample; reconnecting peers remain current until the transport removes them. Discovery is never an authorization or immediate-proximity guarantee.
