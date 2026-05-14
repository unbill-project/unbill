# Implementation

## Structure

- `FileStoreGuard<T>` holds `store: T` (the cloned store) and `_lock_file: std::fs::File` (the sentinel fd); implements `Deref<Target = dyn LedgerStore + 'static>` and `DerefMut`
- `FileLockedStore<T>` holds `store: T` and `lock_path: PathBuf`; implements `LockableStore`
- `lock()` runs `std::fs::create_dir_all` + `OpenOptions::create(true).write(true).open` + `File::lock()` inside `tokio::task::spawn_blocking`, then constructs a `FileStoreGuard` with a clone of the store

## Bounds

- `T: LedgerStore + Clone + 'static + Send` — `LedgerStore` implies `Send + Sync`; `Clone` is needed to hand an owned `T` to the guard; `'static` avoids lifetime issues with `spawn_blocking`

## Dependencies

- `unbill-storage` — `LedgerStore`, `LockableStore`, `StorageResult`
- `tokio` — `task::spawn_blocking` to avoid blocking the async runtime during lock acquisition

## Testing

Tests live in `#[cfg(test)]` at the bottom of `src/lib.rs`. They use `unbill-store-fs::FsStore` as the concrete `T` via a dev-dependency.
