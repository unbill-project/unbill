// Generic OS file-lock–based LockableStore guard. See DESIGN.md before modifying.

use std::ops::{Deref, DerefMut};
use std::path::PathBuf;

use async_trait::async_trait;

use unbill_model::StorageError;
use unbill_storage::{LedgerStore, LockableStore, StorageResult};

/// Exclusive valued lock guard over a `T: LedgerStore`.
///
/// Holds a cloned `T` and an open `std::fs::File` whose OS advisory lock is
/// held for the lifetime of the guard. Dropping the guard closes the file and
/// releases the lock.
pub struct FileStoreGuard<T> {
    store: T,
    _lock_file: std::fs::File,
}

impl<T: LedgerStore + 'static> Deref for FileStoreGuard<T> {
    type Target = dyn LedgerStore + 'static;
    fn deref(&self) -> &(dyn LedgerStore + 'static) {
        &self.store
    }
}

impl<T: LedgerStore + 'static> DerefMut for FileStoreGuard<T> {
    fn deref_mut(&mut self) -> &mut (dyn LedgerStore + 'static) {
        &mut self.store
    }
}

/// Wraps any `T: LedgerStore + Clone` with an OS advisory file lock at
/// `lock_path` and implements [`LockableStore`], handing out
/// [`FileStoreGuard<T>`] on `lock()`.
pub struct FileLockedStore<T> {
    store: T,
    lock_path: PathBuf,
}

impl<T: LedgerStore + Clone + 'static> FileLockedStore<T> {
    pub fn new(store: T, lock_path: PathBuf) -> Self {
        Self { store, lock_path }
    }
}

#[async_trait]
impl<T: LedgerStore + Clone + 'static> LockableStore for FileLockedStore<T> {
    type Guard<'a>
        = FileStoreGuard<T>
    where
        Self: 'a;

    async fn lock(&self) -> StorageResult<FileStoreGuard<T>> {
        let lock_path = self.lock_path.clone();
        let lock_file = tokio::task::spawn_blocking(move || -> std::io::Result<std::fs::File> {
            if let Some(parent) = lock_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let file = std::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .open(&lock_path)?;
            file.lock()?;
            Ok(file)
        })
        .await
        .map_err(|e| StorageError::Serialization(e.to_string()))??;

        Ok(FileStoreGuard {
            store: self.store.clone(),
            _lock_file: lock_file,
        })
    }
}
