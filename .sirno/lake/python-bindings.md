---
core.name: Python Bindings
core.desc: Python access to the shared UniFFI console interface and native library packaging.
core.category:
  - core.concept
core.belongs:
  - workspace-layout
  - unbill-ffi
---

Python clients reuse the UniFFI exports of `unbill-ffi` in `bindings/ffi` through an installable
`unbill` package. Console behavior lives in the native `unbill` Cargo package in `bindings/rust`.

The package contains generated Python bindings and a shared Rust library built
for the host platform. Builds use the workspace-pinned generator and version.
The package exports only the named console API; generated bindings live in a
private `_native` module, and public names do not expose bridge terminology.
Generated bindings and native artifacts stay out of version control.
Wheels are platform-specific and independent of the CPython extension ABI.
Distribution across machines requires bundling external native dependencies.
Python builds fetch locked workspace dependency sources before compilation so
the embedded license catalog can inspect dependencies across all platforms.

The public `Console` type provides explicit in-process and daemon constructors
from the shared `unbill-ffi` interface. In-process consoles run without a separate
daemon and own their data directory until released; daemon consoles connect to
an already-running service. Desktop wheels support both modes.
Synchronous operations block and must be offloaded by asyncio clients;
`sync_once` is awaitable.
Observer callbacks execute on Rust worker threads. Callbacks forward events to
their own thread or event loop before calling console methods.

Python verification covers native loading, values, error propagation,
in-process persistence and directory ownership, operations and callback delivery
without a daemon, and optional daemon-backed integration.

Version tags publish the `unbill` package to PyPI after all platform wheels
build and pass installation tests. Python package versions inherit the Cargo
workspace version; a release tag must match that version exactly.
Linux x86_64 wheels are built in manylinux_2_28 and repaired with auditwheel.
macOS ARM64 wheels are repaired with delocate. Windows x86_64 wheels are repaired
with delvewheel. Repaired wheels retain the Python-independent `py3-none` ABI.
PyPI receives wheels only; repository source remains the source-build entry point.

The dedicated `python.yml` workflow verifies wheels on relevant pull requests,
main-branch pushes, and manual runs. Only version-tag pushes publish.
The publishing job uses PyPI Trusted Publishing in the GitHub `pypi`
environment, with OIDC permission confined to that job. PyPI must authorize
`unbill-project/unbill`, workflow `python.yml`, environment `pypi` before the
first upload. Nightly tags and manual runs do not publish to PyPI.
