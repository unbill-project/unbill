# unbill-store-memory-guard

A generic mutex-based `LockableStore` implementation that wraps any `LedgerStore` type behind a `tokio::sync::Mutex` and exposes it through a valued lock guard.

## Contract

- `LockedStore<T>` holds an `Arc<tokio::sync::Mutex<T>>` where `T: LedgerStore + 'static`
- `lock()` acquires the mutex exclusively and returns a `StoreGuard<T>`
- `StoreGuard<T>` is a valued guard: it owns the `OwnedMutexGuard<T>` and derefs to `dyn LedgerStore + 'static`
- Dropping the guard releases the mutex
- `LockedStore<T>` is `Clone` — cloning shares the underlying `Arc`, not the store

## Rules

- No I/O — mutual exclusion is entirely in-memory via tokio's async mutex
- Does not depend on any concrete store implementation; `T` is a type parameter
