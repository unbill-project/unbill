# Unbill for Python

Unbill is an offline-first expense ledger for households, trips, and other small
groups that already trust each other. The `unbill` package gives Python programs
access to the same Rust console used by Unbill's apps: create ledgers, record
bills, inspect balances and settlements, and sync with other devices. Ledgers
live on member devices, with no hosted account required. Unbill records who
owes whom; it does not move money.

## Install

```sh
python -m pip install unbill
```

Python 3.9 or newer is required. Prebuilt wheels are available for:

| Operating system | Architecture | Requirement |
| --- | --- | --- |
| Linux | x86-64 | glibc 2.28 or newer |
| macOS | Apple silicon | macOS 11 or newer |
| Windows | x86-64 | Windows 64-bit |

The wheels include the native Rust library, so installing one does not require
a Rust toolchain. For other platforms, see the
[source-build instructions](https://github.com/unbill-project/unbill/blob/main/bindings/python/DEVELOPMENT.md).

## Get started

An in-process console runs the Unbill device service and stores its data in the
directory you choose. It does not need a separate daemon:

```python
import unbill

console = unbill.Console.open_in_process("./unbill-data")
try:
    ledger = console.create_ledger("Household", "USD")
    detail = console.ledger_detail(ledger.ledger_id)
    print(detail.summary.name)
finally:
    del console
```

To use an existing `unbill-daemon` instead, connect to its data directory:

```python
console = unbill.Console.open_rpc(unbill.default_data_directory())
```

The daemon must already be running. Both constructors expose the same console
operations. Only one process can own a data directory at a time; release an
in-process console before opening that directory elsewhere.

Most console methods block. From `asyncio`, call them with `asyncio.to_thread`;
`sync_once` is awaitable directly. Errors raise `unbill.ConsoleError`. Observer
callbacks run on Rust worker threads, so forward events to your application's
thread or event loop before calling console methods from a callback.

## Links

- [Source](https://github.com/unbill-project/unbill)
- [Issues](https://github.com/unbill-project/unbill/issues)
- [Build from source and release process](https://github.com/unbill-project/unbill/blob/main/bindings/python/DEVELOPMENT.md)

Unbill is open source under the MIT or Apache-2.0 license.
