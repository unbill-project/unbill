---
core.name: Build Information
core.desc: Application-owned version and UTC build timestamps exposed to connected frontends.
core.category:
  - core.concept
core.belongs:
  - workspace-layout
  - applications
---

`unbill-build-info` only provides the wire data type and display format.
Each application crate and the Apple FFI crate use `build-info-build` in their
own build script and `build-info` in their own code to embed their workspace
version and UTC RFC 3339 timestamp. Git collection is disabled. Build scripts
explicitly choose the current UTC time instead of the reproducible-build
SOURCE_DATE_EPOCH that may be injected by Nix. The shared type
converts each application's generated metadata into the two public fields.
Cargo caches unchanged build-script output; timestamps update when the
application build script reruns. No environment setup or runtime clock lookup
is required. Independently compiled crates or targets can have different timestamps.

LocalAsymChannel receives service build information explicitly from its owning
application, rather than reading metadata compiled into a shared dependency.

The daemon logs its build information to stderr before opening storage or networking.
The asymmetric channel exposes a read-only service build-info method through
Local, RPC, and HTTP. RPC forwards the serving channel's information and HTTP
returns the server's compiled information at authenticated GET /api/v1/build-info.
Console delegates this query to its connected channel.

Frontends display their own compiled information and the connected service's
information separately in application/device settings. Desktop identifies the
service as Daemon, mobile as In-process service, and the browser as Server.
Query failures display Service version unavailable, without substituting the
client's version or blocking normal bootstrap. Apple exposes this through UniFFI.
Command-line applications provide --version without opening storage or connecting.

Version information appears as a compact footer at the bottom of settings, after the settings controls.
