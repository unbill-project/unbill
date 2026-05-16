# unbill Architecture

This document describes the core conceptual model of unbill. It focuses on the **wishful layer**—what each concept is, why it's designed this way, and how concepts relate to each other. Implementation details (traits, wire protocols, fields, API shapes, etc.) are left to engineering documents.

______________________________________________________________________

## 0. Why, not how

Before describing the model, it helps to state the design intent. The architecture that follows is a consequence of these positions, not an arbitrary set of structural choices.

### Why P2P

Ledger applications today (Splitwise, etc.) require a central server that holds every user's account data. This model carries costs that don't disappear: the operator can read all ledgers, must be trusted to never go down or lose data, and decides who can use the service. For a friend / family ledger, none of these costs buy the user anything meaningful—the data is already shared among a small trusted circle, and the operator adds no value beyond storage.

unbill is P2P so that **the data lives where the trust already lives**—on the devices of the people who share the ledger. No third party holds a copy, no third party gates access, no third party can disappear.

### Why CRDT

A P2P ledger must reconcile concurrent edits from multiple devices, often after long offline periods. There are two routes: design a custom merge protocol per data type, or use a general-purpose convergent data structure. CRDTs (specifically Automerge) give convergence as a property of the data structure itself, leaving the application free to focus on semantics rather than merge mechanics.

unbill uses CRDT because **convergence under arbitrary network conditions should be an invariant, not a feature to engineer per operation**.

### Why device and console

Compressing storage and display into a single role means every UI carries a full replica of the state. This works for a single-UI setup but breaks quickly: opening two UIs on the same machine fragments the data, causes sync conflicts, and leaves the two views inconsistent; accessing the ledger from a remote location requires remote-desktoping into the local UI or isn't possible at all.

unbill separates the two concerns into **device** (focused on storage) and **console** (focused on display) to **unlock deployment flexibility**:

- The same machine can run a Tauri app, a TUI, and a browser-based web frontend simultaneously—all attached to the same device and viewing the same data
- A remote web console can connect to a device at home without depending on any local UI being open
- Switching UI form factors (desktop during the day, phone at night) is transparent to background sync
- A shared family device can host consoles from every family member at once

The cardinality is naturally asymmetric: a machine typically needs only one device, but can host multiple consoles concurrently. This split lets each role live where it thrives, and lets unbill cover the full range of deployment shapes—single user, family-shared, remote access.

### Why two kinds of channels

The sync protocol between two devices is fundamentally different from the protocol between a device and a console. Device-to-device sync is symmetric op exchange; device-to-console interaction is request/response plus subscription, and also carries control operations. Trying to unify these under one protocol either over-complicates the simple case or under-serves the complex one.

unbill separates **sym channel** (device ↔ device, data plane only) and **asym channel** (device ↔ console, data plane + control plane), so each channel can be exactly as complex as its job requires.

### Why peer-equal trust

P2P systems cannot enforce fine-grained permissions without resorting to cryptographic schemes that complicate every operation (forward-secret group keys, re-encryption on revocation, etc.). The complexity is justified only when participants are mutually suspicious.

For friend / family ledgers, participants are not mutually suspicious—they are friends and family. The actual threat model is "what if a stranger gets in," not "what if Alice secretly changes Bob's bill." unbill establishes trust once at invite-acceptance time and treats all members as equally privileged thereafter. **The system optimizes for the cooperation that's actually happening, not for adversarial scenarios that aren't.**

### Why amend instead of mutation

CRDT auto-merge is correct as an algorithm but often wrong as a business semantic. When two people independently change the same bill, the algorithm picks one (or merges fields), but neither result reflects the human intent—each editor had a reason, and the right outcome usually requires understanding both reasons.

unbill makes the bill list append-only and expresses modifications as new bills pointing to old ones. Concurrent modifications become visible as conflicts in the bill graph, and merging is a human decision encoded as a merging bill. **The algorithm guarantees state convergence; humans decide what the state should mean.**

### Why one Rust codebase across platforms

Maintaining separate native implementations per platform (Swift for iOS, Kotlin for Android, TS for web, etc.) is the industry norm, but each implementation drifts subtly, multiplies the bug surface, and slows feature velocity. For a small team or solo developer, it's prohibitive.

The Rust + WASM + Tauri + Leptos stack lets unbill share the console logic, the UI code, and the data model as one codebase that compiles to every target. **The platform reach is achieved by leverage, not by headcount.**

______________________________________________________________________

## 1. Device

A **Device** is a full-fledged participant in unbill's P2P network. It owns ledger data, holds a stable identity, and synchronizes with other devices.

- Persists CRDT state (the full Automerge document and history) to local storage
- Holds a stable cryptographic identity (NodeId, Ed25519)
- A first-class citizen of the P2P network, addressable by other devices
- Holds a file lock on startup to ensure only one instance runs per data directory
- Actively maintains synchronization loops with other devices after startup

Typical forms: desktop daemons, always-on processes on home NAS / VPS, background tasks within mobile apps.

A device is a **logical role**. A single machine typically runs one device, but the role is decoupled from any specific process or binary—the same role can be embedded in a Tauri app, run as a standalone daemon, or coexist with a console in a mobile process.

______________________________________________________________________

## 2. Console

A **Console** is a user-facing surface that operates a device. It holds no persistent ledger state and exists only for the duration of a session.

- Holds a transient identity, alive only for the session
- Maintains small amounts of in-memory state during a session (connection sessions, subscription caches, optimistic updates, etc.), discarded when the process exits
- Attaches to a device to function—operating ledgers, submitting bills, viewing state
- Multiple consoles can concurrently attach to the same device

Typical forms: front-end code in a web browser, the front-end layer of a Tauri app, TUI clients.

**Local preferences**: the "non-persistent" property of a console applies specifically to ledger state. A console may still use whatever storage the host environment provides (filesystem, browser localStorage, etc.) to save local preferences—such as the most recently used user, UI settings, recently opened ledgers, etc. These preferences are independent of ledger data, serve local UX, and their loss does not affect collaboration correctness.

### Identity

Technical identity (NodeId) belongs to devices. A console participates as the operating surface of a device, with no identity of its own at the network layer.

______________________________________________________________________

## 3. Channel

A **Channel** is the abstraction of a communication relationship—it describes who talks to whom and what responsibilities the conversation carries.

Channels come in two kinds, corresponding to the two possible pairings. Each kind has its own independent implementation.

### Symmetric channel (sym channel)

Occurs between **device ↔ device**.

- Both ends are peers, each holding complete CRDT state
- Single responsibility: converge the CRDT state on both sides—pure data plane
- True P2P communication in spirit

### Asymmetric channel (asym channel)

Occurs between **device ↔ console**.

- The two ends are unequal. The device is the source of truth; the console is its projection plus an entry point for operations
- Carries both a data plane and a control plane:
  - **Data plane**: the console subscribes to ledger state projections and submits modifications to the ledger
  - **Control plane**: creating ledgers, generating and accepting invites, managing peers, triggering sym sync, etc.
- A single device can host asym channels for multiple consoles concurrently

The asym channel has broader responsibilities than the sym channel—it's both a state synchronization conduit and the overall channel through which a console operates a device.

### Semantically isomorphic, implementationally independent

The two channels are isomorphic at the CRDT semantic level: both are "two views reaching agreement through information exchange." The differences:

- sym holds complete history on both ends, runs a standard CRDT sync protocol, and stays within the data plane
- asym holds the authoritative replica and full history on one end and a transient projection on the other, and additionally carries the control plane

Their concrete implementations are independent.

______________________________________________________________________

## 4. Ledger

A **Ledger** is unbill's core business object—an independent account book recording bills, users, balances, and so on.

Each ledger is, at the bottom, an independent Automerge document. A device can hold multiple ledgers.

### Members

A ledger's members are a set of devices (identified by NodeId). Membership propagates naturally through the CRDT—when a new member joins, their NodeId becomes visible to other members through the synchronization of the ledger document.

### Trust model

**Full peer-equal trust inside a ledger**: all devices that have joined a ledger have equal read/write permissions on its data.

- All members hold symmetric read/write capabilities over the ledger
- The legitimacy of an operation is determined solely by whether the submitter is a member; no op-level validation is required
- Operation provenance is opaque to the system—the ledger records "what happened," not "who did it"
- Trust is established once and for all at the moment an invite is accepted; trust breakdown is a social matter, resolved outside the product

This choice aligns with the foundational philosophy of P2P CRDT systems (Automerge, Syncthing, Git collaborators all use the same model) and fits unbill's product positioning for friend / family ledgers.

### Ledger join flow

A new member joining a ledger involves both kinds of channels:

1. Member A (already in the ledger) generates an invite via the control plane of their asym channel
1. The invite is delivered out-of-band to B (via QR code, link sharing, etc.)
1. B submits the invite via the control plane of their asym channel
1. B's device contacts A's device over the sym channel, presenting the invite credential during handshake
1. A verifies it, sym sync begins, and B's join is complete

In other words, **invite generation and acceptance belong to the control plane (asym), while invite redemption belongs to the data plane handshake (sym)**.

______________________________________________________________________

## 5. User

A **User** is the **accounting dimension** inside a ledger, representing "who paid" and "who shares the cost."

A user is purely business data, decoupled from any technical identity:

- A user is a label inside a ledger, independent of device, NodeId, or login identity
- A real person may correspond to multiple users (recording bills on behalf of friends or family)
- Multiple real people may share a single user (a merged-account view)
- Any member can operate any user

**User operation is fully open to all ledger members**—any member can submit a bill like "Alice paid 100," and the system accepts it as-is. This is consistent with the "joined-equals-trusted, actor-unrecorded" trust model:

- Ledger-level access control: membership determines a device's write permission
- User-level semantics: a user is just a label inside bill content, given meaning by social convention

A user is essentially a form of **role-playing**—the user in the ledger is a customary reference, with the actual operator settled by social convention. This flexibility is deliberate and fits the friend / family ledger positioning.

______________________________________________________________________

## 6. Bills and the Amend Model

A **Bill** is a single accounting record in a ledger. The bill list is **append-only**—once submitted, a bill is never modified in place.

### Amend means appending

A "modification" to a bill is expressed by **creating a new bill that points to the old one**, called an **amend**. The original bill stays untouched; the new bill (the amending bill) carries the relation "I am an amend of X." Deletion is a special amend (tombstone) pointing to the deleted bill.

The "current account state" at any moment is resolved from the bill list: follow amend chains to find the leaf (latest version) of each bill; tombstones indicate deletion; the remaining leaves form the current effective bills.

### Conflicts made explicit

Since the bill list is append-only, at the CRDT layer every bill is a legitimate addition. **Conflicts are defined at the business layer**: when two or more bills concurrently amend the same older bill, it means two devices, while offline, made independent and different modifications to the same record.

The system **hands the conflict to the user for resolution**: it explicitly marks the conflict in the UI and guides the user to create a **merging bill**—a new bill that amends multiple conflicting versions at once, with content decided by the user.

A merging bill is itself an ordinary bill and can be further amended or once again be in conflict. The full bill relation graph forms a DAG, isomorphic to Git's commit graph.

### Design implications

This model **hands the decision of "how to merge concurrent modifications" from the algorithm layer to the user layer**:

- The CRDT algorithm ensures the bill list converges; business semantics are handled above it
- The business layer makes semantic conflicts visible, traceable, and resolvable by humans
- History is fully preserved; every amend is auditable
- Unresolved conflicts can be left pending—users resolve them at their own pace

This aligns with unbill's overall design philosophy: **P2P + CRDT provides the underlying consistency; users drive the business-layer resolution**.

______________________________________________________________________

## 7. Sync behavior

A device continuously maintains synchronization with other devices. The triggers come from several sources:

- A peer comes online
- New local ops on a ledger
- Periodic fallback
- Explicit user request
- Platform-specific wakeups (mobile background tasks, etc.)
- Network state changes

These signals converge into a "sync trigger" event stream; the sync engine decides who to sync with and which ledgers to sync based on current state.

### Platform differences

The availability of a device varies across deployment forms:

- **Desktop / server**: near always-on; sync proceeds nearly continuously
- **Mobile**: intermittent. The device syncs normally while the app is in the foreground; gets frozen shortly after entering background; if killed by the system, state remains on disk and is caught up by peer sync on the next startup

This difference is a reality of deployment, while the model stays uniform: all devices are conceptually equal sync nodes.

______________________________________________________________________

## 8. Event propagation

Data flows from device to UI in two hops:

```
device ──[asym channel]──► console ──[host-specific mechanism]──► UI
```

**First hop** (device → console): the console subscribes through the asym channel to the ledgers it cares about; the device pushes events through the channel when state changes.

**Second hop** (console → UI): the console is the **translation layer and hub** for events—it translates low-level CRDT changes into materialized views, aggregates raw events into higher-level semantic events, and exposes them to the UI in the most natural way for the host environment.

The UI always sees usable views plus high-level events; the CRDT details are absorbed by the console.

______________________________________________________________________

## 9. Multiple frontends

unbill is planned to support multiple frontends, all of which play the role of a console. **All graphical frontends use Leptos uniformly**:

- **Web**: Leptos compiled to WASM, running in a browser
- **Desktop**: Tauri, hosting a Leptos frontend
- **Mobile**: Tauri Mobile, hosting a Leptos frontend; a device runs in the same process
- **TUI**: Ratatui, running in a native Rust process with immediate-mode terminal rendering
- **CLI**: command-line tool, suited to one-shot invocations, scripts, and automation

The entire frontend stack shares a single Rust codebase—the console core and Leptos UI are both Rust, compilable to WASM for the browser or linkable as native code into Tauri / Tauri Mobile / TUI / CLI processes. TUI and CLI are two distinct apps: TUI is an interactive terminal interface that uses the same console core; CLI provides a command-line entry point suited to scripts and automation.

______________________________________________________________________

## 10. Deployment topologies

The model is decoupled from deployment. **The same unbill installation runs under multiple topologies**, with the model staying stable; only the physical distribution differs. A few typical topologies:

### Single-process colocated

Inside a mobile app, a device and a console coexist as two tasks within the same process; the asym channel is in-process memory communication. This fits the sandbox model of mobile platforms—iOS requires apps to run in a single process, and Android practice prefers the same shape.

### Local two-process

The standard form on desktop: the device runs as a standalone daemon process in the background, while Tauri / TUI / CLI run as separate processes communicating with it over local IPC. The daemon can continue to maintain sync after the UI is closed. Multiple consoles on the same machine can concurrently connect to the same device.

### Remote access

The user runs a device on a home server / VPS and connects to it from anywhere via the web frontend or other consoles.

### Multiple devices

A user may own multiple devices simultaneously (laptop + VPS + phone), all connected to one another via sym channels. This is common and is a target scenario for unbill's design.

______________________________________________________________________

## 11. Design stance

These are the deliberate trade-offs at the model layer—the stance of unbill, defining the scenarios the system is built to serve:

- **Social trust over cryptographic defense**: a member who joined the ledger is treated as trusted; trust is established by human relationships, and the system focuses on smooth collaboration among trusted members
- **Roles over identities**: user is a labeling dimension inside a ledger, given meaning by social convention, decoupled from login identity
- **State over provenance**: the ledger records "what happened"; "who did it" is carried by social context
- **Eventual consistency as a core feature**: CRDT convergence ensures the ledger reaches agreement under any network conditions
- **Best-effort sync**: intermittent connectivity is the norm in P2P networks; sync makes its best effort within connection windows
- **Equal members**: all members of a ledger have equal permissions over its data
- **Offline-first**: mobile backgrounding, network interruptions, and device sleep are normal states; data catches up naturally through peer sync once reconnected

These choices make P2P + CRDT viable and clean for the friend / family ledger scenario. Each item corresponds to a specific simplification, and together they define unbill's product positioning.

______________________________________________________________________

## 12. Glossary

| Term | Meaning |
|---|---|
| Device | A full P2P participant; persists CRDT state and holds a NodeId |
| Console | A user-facing surface that operates a device; holds only transient state |
| Channel | The abstraction of a communication relationship |
| Sym channel | The communication relationship between two devices |
| Asym channel | The communication relationship between a device and a console |
| Ledger | An independent account book, an Automerge document at its core |
| User | A ledger-internal accounting dimension; a role label independent of technical identity |
| Bill | A single accounting record in a ledger; append-only |
| Amend | Expressing a modification by appending a new bill that points to an older one |
| Amending bill | A new bill that amends an older bill |
| Merging bill | A bill that simultaneously amends multiple conflicting versions, created by the user |
| Invite | A one-time credential for joining a ledger |
| NodeId | A device's Ed25519 public key identity |
