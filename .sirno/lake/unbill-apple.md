---
core.desc: The native SwiftUI frontend for Apple platforms and the macOS release app.
core.name: Unbill Apple
core.category:
  - core.concept
core.belongs:
  - frontend-ui
  - applications
core.refines:
  - ui-shared-model
---

`unbill-apple` is the native SwiftUI frontend for Apple platforms.
It supplies the macOS release app, replacing the Tauri macOS bundle in the release pipeline.
Tauri continues to supply the Linux and Windows desktop bundles.
It targets iPhone and iPad as one universal build and macOS through Mac Catalyst.

The app is backed by the real Rust core.
`ConsoleClient` is the single Swift boundary, an async protocol over the `UnbillConsole` orchestration surface.
`RustConsoleClient` is its only implementation: an actor that calls `unbill-ffi` and runs the synchronous bridge off the main thread.
There is no mock backend.
Listing ledgers only reads persisted state. A fresh installation starts with an empty
ledger list; the app does not automatically create sample ledgers, users, or bills.
On iOS and Simulator the app calls `Console.openInProcess` to open a SQLite device service
in the app's data directory, skipping the desktop ownership lock.
On macOS and Mac Catalyst it calls `Console.openRpc` to connect by RPC to an already-running daemon
using the shared desktop data-directory default and socket override.
Startup failures display their error; daemon installation and startup follow the desktop model.
Existing flat-file data remains untouched and is not automatically imported.
The new-ledger currency picker obtains the complete Rust currency catalog through `ConsoleClient`
and UniFFI, sorted by currency code, rather than maintaining a Swift shortlist.

`unbill-ffi` is a Cargo workspace member that exposes the shared `Console` interface to Swift with UniFFI.
It mirrors the aggregated command granularity of the Tauri bridge rather than the raw console methods:
bootstrap, ledger detail, create ledger, create and import users, save bill, resolve conflict,
create invitation, join, sync, and a `ServiceEvent` callback stream.
The DTO assembly is ported from `unbill-tauri`; a shared `unbill-shell` crate would remove that duplication.

The project lives at `apps/unbill-apple`, outside the Cargo workspace.
An `xcodegen` `project.yml` defines one application target for iOS device, simulator, and Mac Catalyst.
It declares a shared `unbill` scheme so fresh CI checkouts can build without opening Xcode.
`build-rust.sh` compiles `unbill-ffi` into `UnbillCore.xcframework` and generates the Swift bindings.
Each target supplies its own generated Swift bindings, C header, and module map.
Swift conditional compilation selects the matching generated source, and each
XCFramework slice carries its matching headers. iOS device and Simulator bindings
omit the daemon constructor entirely; Mac Catalyst bindings expose both constructors.
Build-time Swift checks accept in-process construction on every target and require
daemon construction to fail on iOS device and Simulator while succeeding on Catalyst.
The generated project, Swift bindings, xcframework, `Info.plist`, and docs projection stay out of version control.

`build-apple-dmg.yml` builds an Apple Silicon Catalyst DMG on macOS CI.
It pins Xcode 26.6 on the macOS latest runner for both Rust and Swift builds.
Building against the 26-series SDKs enables the system's Liquid Glass design on macOS Tahoe;
the deployment target remains compatible with older macOS releases.
It is reusable-only, with read-only repository permissions and no standalone event triggers.
Only nightly and version-tag releases enable its opt-in job in the shared build workflow.
Direct manual runs of the shared build workflow do not build the DMG.
The Rust build's `--catalyst-only` option builds only the Mac slice for this workflow;
the default build still includes iOS device and simulator slices.
The workflow uploads `unbill-macos-aarch64.dmg` in a `binaries-*` artifact for the existing release collector.
It packages `Unbill.app` to preserve the Homebrew cask contract and takes its version from the Rust workspace.
The app is ad-hoc signed; Developer ID signing and notarization are not configured.

The screens follow the shared UI model adapted to Apple idioms.
A `NavigationSplitView` shell shows a ledger list and, on selection, ledger detail with people, conflicts, bills, and settlement.
Mac Catalyst uses three columns: ledgers, ledger content, and the bill editor.
The ledger sidebar uses native split-view resizing with a requested width range
of 180–360 points and a 240-point default; available window space may constrain it.
New bills and amendments open inline in the third column; iOS presents the editor as a sheet.
Changing ledgers clears the editor; saving refreshes ledger content and the ledger list.
Wired flows: create ledger; add and import known people; add a bill with Rust-computed settlement; resolve amendment conflicts;
create an invitation as a QR code; join by pasting or scanning a code; and a devices screen with manual per-peer sync.
Live QR scanning uses VisionKit only on supported iOS hardware.
Mac Catalyst builds exclude the scanner and its presentation; joining on Mac uses invitation-link entry.
Peer discovery and sync use the local network, so the app declares Local Network, Bonjour, and camera usage.

Two toolchains must cooperate to build for Apple.
The devenv shell repoints `DEVELOPER_DIR` and `SDKROOT` at a Nix Apple SDK and injects Nix compiler and linker variables.
`xcbuild.sh` scrubs them and selects the real Xcode for `xcodebuild` and `xcodegen`.
`build-rust.sh` links host artifacts with the Nix toolchain, then cross-compiles the iOS targets with Xcode's clang,
because the core pulls C crypto through `unbill-device` and no single global SDK satisfies both.
Swift binding generation invokes the host binary built in the first phase directly,
so it does not rebuild host dependencies under the cross-compilation environment.
The app builds and runs on the iOS Simulator, Mac Catalyst, and a real iPhone with automatic code signing.

Ledger detail DTOs include the console-generated emoji fingerprint. A Ledger
Settings sheet shows it with wrapping, text selection, and a copy action.
The Swift console boundary exposes ledger-update events through AsyncStream;
ledger detail reloads on matching updates and resync notifications.

Joining requires the name of this joining device in the ledger. Device names
are read from the ledger device records, without a local label lookup.

People offers Archive and Restore actions. Archived people and bill participant
choices appear in collapsed disclosure groups; new bill defaults use active people.

Application Settings displays client version and UTC build timestamp alongside the connected daemon or in-process service build, obtained through UniFFI, as a compact footer at the bottom.

Application Settings links to a searchable Open Source Licenses list with Unbill
first. Software details show version, license expression, repository, source
location and selectable original legal texts. Standalone UniFFI functions expose
the local catalog and selected software without a console connection. The Swift
loader reads them off the main actor and renders only selected software details.
The startup failure screen also offers the license browser without a device
connection. Apple build workflows exercise the Foundation-only search model.

The Swift Apple iOS app is built unsigned by `build-apple-ios.yml` on macOS CI.
Nightly and version releases enable `build_apple_ios`; manual builds can opt in.
The Rust build script's `--ios-only` option builds the device slice and generates
Swift bindings. Xcode builds Release for generic iOS with signing disabled.
The workflow packages `Payload/unbill.app` as `unbill-apple-ios.ipa` in a
`binaries-apple-ios-aarch64` artifact, collected by the GitHub release workflow.
Users sign the IPA themselves for sideloading. No signing secrets are required.

The XcodeGen Info.plist version fields expand MARKETING_VERSION and CURRENT_PROJECT_VERSION, so release IPA metadata matches the workspace version.

Apple's shared bill editor supports new bills and amendments, multiple payers,
equal or custom positive integer weights on both sides, and live amounts from
UnbillConsole's verified splitting algorithm through UniFFI.
Explicit Equal shares / Custom shares segmented controls select each side's mode;
custom weights use labeled, bordered numeric inputs and survive mode changes.
Payer selection uses square checkboxes with whole-row activation and accessible
selection state on iOS and Mac Catalyst, rather than switches.
UniFFI allocates a real bill ID when a draft opens; preview and save use that same ID,
so displayed amounts match saved amounts. Amendments allocate a new ID rather than
reusing the original bill ID. Amendments preserve archived
participants and original weights. Decimal input rejects excess precision,
nonpositive amounts, and overflow rather than truncating it.
Bill rows show participants and timestamps with neutral text styling. On iOS,
swiping left reveals an Edit action. Mac Catalyst bill rows are selectable across
the whole row and show a large neutral trailing chevron that opens the third column;
a context menu also opens the editor. Native `List(selection:)` tracks the open
bill and provides the system selection highlight and keyboard selection. Closing
the editor clears selection; people, conflicts, and settlement rows are not selectable.
The editor omits explanatory save and amendment notices. Conflict choices show
weights, timestamps, and ancestor summaries. Ledger Settings includes authorized
peer devices and sync actions. Appearance supports system, light, and dark modes.

Ledger lists and detail subscribe to service updates, refresh on foreground entry,
and expose manual refresh/retry. Failed reloads clear prior data and display errors;
request generations prevent older responses from overwriting newer results.
The app registers the unbill URL scheme and opens invitation links in the join sheet.
The sheet requires this joining device's ledger name. Startup errors offer retry.
Startup failure uses ContentUnavailableView's actions area to keep a prominent
system Retry button directly below the error description, with Return activation.
Apple CI runs Foundation-only amount boundary tests before building Rust and Swift.

Plus toolbar controls contain only creation actions: New Ledger, Add Person, and
Add Bill. Join Ledger, Invite via QR, and Ledger Settings have separate labeled
toolbar controls with their own icons.

Peer sync uses an asynchronous UniFFI export, running network work on the FFI-owned
Tokio runtime and yielding the Swift console actor while it waits. Local ledger
queries remain available during sync. The Devices sheet disables Done and interactive
dismissal while Sync or Sync All is running, restoring both after success or failure.
Closing the sheet triggers a fresh ledger-list load; completion events refresh the UI.
Each device row has one Sync/Retry button on the right, with an in-place spinner
while active and an accessible Syncing label. Successful completion appears under
the device name as a checkmark and Sync complete with its local completion time.
This confirmation is hidden during retry or after failure; failures show their error
under the device name. Results are scoped to this sheet session, without separate status
badges beside the button. Sync All reports success/failure counts
below the device list. Completion status follows the sync call's actual result.

Global Devices and Ledger Settings reuse the same peer-device row, including
name, truncated secondary Node ID, ledger names, bordered Sync/Retry button,
in-place progress indicator, completion timestamp, and per-peer errors. Both
sheets guard duplicate sync requests and disable dismissal while syncing.

Add Person always displays From Other Ledgers with explicit Import actions,
excluding people already in the current ledger and sorting candidates by name.
Loading, empty, and failed states remain visible; failed loads offer Retry and
the sheet supports refresh. Saving guards repeat actions and prevents dismissal.

Ledger Settings provides Archive Ledger, and the ledger sidebar groups archived ledgers in a collapsed native DisclosureGroup with navigation rows. The Swift console protocol exposes setLedgerArchived and summaries carry archived. Archived rows retain explicit opening actions so users can open their details without restoring them. Refresh or ledger update notifications clear selection and detail/editor context when an active ledger becomes archived, but retain an explicitly opened archived ledger across subsequent refreshes. Archive requests disable settings dismissal while in progress and report failures without clearing data.

Ledger list rows only open ledger details; restoration belongs in Ledger Settings. Explicitly opened archived ledgers remain selected across refreshes.

Clicking a ledger row replaces its trailing chevron with a progress indicator and
fetches the complete ledger detail through ConsoleClient and UniFFI before selecting
or navigating. Duplicate opening requests are ignored while the fetch runs; failure
keeps the list visible and presents the error. The destination starts with this
loaded detail and skips a redundant initial fetch, while later updates and refreshes
still reload it. Mac bill saves discard the initial snapshot before refreshing.
Compact iPhone layouts use an explicit NavigationStack with ledger-ID destinations;
iPad retains two-column navigation and Mac Catalyst retains three columns. The stack
path and ledger selection stay synchronized for back navigation and archiving.
The detail screen keeps a stable full-size container for content and reload errors.

The application Settings gear belongs to the ledger-list view inside the navigation
container. iOS explicitly places it in the trailing navigation bar; Mac Catalyst
uses automatic placement. RootView owns the settings sheet presentation.

Application Settings uses native Form sections for Appearance and the Open Source
Licenses navigation row, retaining the system row styling and disclosure indicator.
Selectable build information remains a separate compact footer below the form.

Global Devices is opened from an application Settings form row; the ledger-list
toolbar no longer presents it. Sync completion and device-sheet dismissal refresh
the ledger list. Settings and its device sheet apply the stored appearance choice
at their presentation boundaries so switching Light, Dark, or System updates them
while open.

The Local Network Bonjour declaration includes _irohv1._udp, matching the Iroh
mDNS companion crate's service name.
The app also declares Bluetooth usage and links CoreBluetooth for the Rust BLE
transport. No background Bluetooth mode is declared.
Physical iOS builds also request the multicast entitlement because this service
uses UDP multicast sockets directly. Device signing must use a provisioning
profile that grants this capability; the entitlement is not applied to Catalyst.
