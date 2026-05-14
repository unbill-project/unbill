# Implementation

## Structure

- `StoreGuard<T>` wraps `tokio::sync::OwnedMutexGuard<T>` and implements `Deref<Target = dyn LedgerStore + 'static>` and `DerefMut`
- `LockedStore<T>` holds `Arc<tokio::sync::Mutex<T>>` and implements `LockableStore`
- `lock()` calls `Arc::clone(&self.0).lock_owned().await` to produce an owned guard with no borrow from `self`, satisfying the `'a`-free guard type in the GAT

## Bounds

- `T: LedgerStore + 'static` — `LedgerStore` already implies `Send + Sync`; `'static` is required by `lock_owned()`

## Dependencies

- `unbill-storage` — `LedgerStore`, `LockableStore`, `StorageResult`
- `tokio` — `sync` feature for `Mutex` and `OwnedMutexGuard`

## Testing

Tests live in `#[cfg(test)]` at the bottom of `src/lib.rs`. They use `unbill-store-memory::InMemoryStore` as the concrete `T` via a dev-dependency, keeping the production code store-agnostic.
