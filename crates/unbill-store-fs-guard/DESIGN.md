# unbill-store-fs-guard

A generic OS file-lock–based `LockableStore` implementation that wraps any `T: LedgerStore + Clone` behind an exclusive advisory file lock and exposes it through a valued guard.

## Contract

- `FileLockedStore<T>` holds a `T` and a `PathBuf` pointing to the lock sentinel file
- `lock()` acquires an exclusive OS advisory lock on the sentinel file (blocking, run via `spawn_blocking`) and returns a `FileStoreGuard<T>`
- `FileStoreGuard<T>` owns a cloned `T` and the open `std::fs::File` that holds the lock; it derefs to `dyn LedgerStore + 'static`
- Dropping the guard closes the file descriptor, which releases the OS lock automatically
- The sentinel file is created on first `lock()` and persists across calls; only the OS lock on it is released on drop

## Rules

- File locking is advisory: all participants must use this crate for mutual exclusion to be effective
- `T: Clone` is required so the guard can own an independent copy of the store
- This crate is not WASM-compatible; file locks require OS support
- The caller chooses the sentinel file path — this crate enforces no naming convention
