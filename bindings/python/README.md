# Unbill Python bindings

After CI publishes a release, install it with `python -m pip install unbill`. Wheels
target Linux x86_64 (glibc 2.28 or newer), macOS ARM64, and Windows x86_64.
The daemon is installed separately and must be running before opening a console.

The Python package reuses the Rust console interface through UniFFI. Build it
from the repository with Cargo and Python 3.9 or newer:

```sh
python3 bindings/python/build.py
python3 -m pip install ./target/python/debug
```

For an optimized build, add `--release` and install `./target/python/release`.
The output follows Cargo's target directory if you override it. Generated
bindings and the native library stay under that directory and are not committed.
The generator is built from the same pinned workspace dependencies as the library.

The package includes a native shared library for the build host. Wheels are
platform-specific (`py3-none-<platform>`), and require no Rust toolchain at runtime.
To build a wheel for distribution:

```sh
python3 -m pip wheel --no-deps ./target/python/release -w ./target/python/wheels
```

Build separately for each supported operating system and architecture. Linux
distribution may additionally require wheel repair with auditwheel; macOS may
require delocate when the build links external shared libraries. The script
builds for the Rust host target even if a cross-compilation target is configured.

Start `unbill-daemon` before opening a desktop console. The data directory and
`UNBILL_SOCKET` override follow the existing desktop configuration.

```python
import unbill

console = unbill.FfiConsole.open(unbill.default_data_directory())
ledger = console.create_ledger("Household", "USD")
detail = console.ledger_detail(ledger.ledger_id)
print(detail)
```

Records, enums, errors, and methods are generated from the same Rust exports
used by Swift. Rust errors raise `unbill.FfiError`. Most methods are synchronous
and block; use `asyncio.to_thread` when calling them from an asyncio application.
`sync_once` is async and can be awaited directly.

Observers subclass `unbill.FfiConsoleObserver` and implement `on_event(self, event)`.
Keep the subscription returned by `console.observe(observer)` alive, then call
`subscription.cancel()` when finished. Callbacks run on Rust worker threads:
enqueue events or use `loop.call_soon_threadsafe` to forward them to the owning
event loop before calling console methods.

Verify the generated package without installing it:

```sh
PYTHONPATH=target/python/debug python3 bindings/python/test_bindings.py
```

For daemon and observer verification, start a daemon with an isolated
`UNBILL_DATA_DIR`, then set `UNBILL_PYTHON_TEST_DIR` to that same directory when
running the tests. Use a temporary directory because this test creates a ledger.

## CI and PyPI publishing

`.github/workflows/python.yml` builds, repairs, and tests wheels on relevant pull
requests, pushes to `main`, and manual runs. Pushing a `v*` tag also publishes to
PyPI after every platform passes. The tag must exactly match `v` followed by the
Cargo workspace version. Package versions come from the workspace; nightly tags
and manual runs do not publish. Rerunning a release skips files already on PyPI.

Linux wheels are built in manylinux_2_28 and repaired with auditwheel. macOS wheels
use delocate, and Windows wheels use delvewheel to bundle external dependencies.
CI installs the repaired wheels on Python 3.9 and 3.14 before uploading artifacts;
Linux also tests Python 3.11. PyPI receives wheels only, so unsupported platforms
use the repository build instructions above.

Publishing uses [PyPI Trusted Publishing](https://docs.pypi.org/trusted-publishers/using-a-publisher/).
Configure it once before pushing the first release tag:

1. Create a GitHub Actions environment named `pypi` in `unbill-project/unbill`.
   Restrict its deployment tags to `v*`.

1. In your PyPI account, open [Publishing](https://pypi.org/manage/account/publishing/)
   and add a pending GitHub publisher with these values:

   | Field | Value |
   |-------|-------|
   | PyPI project name | `unbill` |
   | Owner | `unbill-project` |
   | Repository | `unbill` |
   | Workflow filename | `python.yml` |
   | Environment | `pypi` |

   If the PyPI project already belongs to you, add the publisher under that
   project's Publishing settings instead. A pending publisher creates the
   project on its first successful upload; it does not reserve the name.

1. Push a version tag through the existing `cargo release` process.
   The Python workflow publishes alongside the existing application releases.

No PyPI API token or GitHub repository secret is required. Only the publishing
job receives `id-token: write`; build and pull-request jobs have read permissions.
