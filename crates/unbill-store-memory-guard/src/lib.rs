// Generic mutex-based LockableStore guard. See DESIGN.md before modifying.

use std::ops::{Deref, DerefMut};
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::OwnedMutexGuard;

use unbill_storage::{LedgerStore, LockableStore, StorageResult};

/// Exclusive valued lock guard over a `T: LedgerStore`.
///
/// Holds a `tokio` owned mutex guard and derefs to `dyn LedgerStore + 'static`.
pub struct StoreGuard<T>(OwnedMutexGuard<T>);

impl<T: LedgerStore + 'static> Deref for StoreGuard<T> {
    type Target = dyn LedgerStore + 'static;
    fn deref(&self) -> &(dyn LedgerStore + 'static) {
        let r: &(dyn LedgerStore + 'static) = &*self.0;
        r
    }
}

impl<T: LedgerStore + 'static> DerefMut for StoreGuard<T> {
    fn deref_mut(&mut self) -> &mut (dyn LedgerStore + 'static) {
        let r: &mut (dyn LedgerStore + 'static) = &mut *self.0;
        r
    }
}

/// Wraps any `T: LedgerStore` in an `Arc<tokio::sync::Mutex<T>>` and
/// implements [`LockableStore`], handing out [`StoreGuard<T>`] on `lock()`.
///
/// `Clone` shares the underlying `Arc`; it does not clone the store.
pub struct LockedStore<T>(Arc<tokio::sync::Mutex<T>>);

impl<T> Clone for LockedStore<T> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl<T: LedgerStore + 'static> LockedStore<T> {
    pub fn new(store: T) -> Self {
        Self(Arc::new(tokio::sync::Mutex::new(store)))
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl<T: LedgerStore + 'static> LockableStore for LockedStore<T> {
    type Guard<'a>
        = StoreGuard<T>
    where
        Self: 'a;

    async fn lock(&self) -> StorageResult<StoreGuard<T>> {
        Ok(StoreGuard(Arc::clone(&self.0).lock_owned().await))
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use unbill_storage::LockableStore;
    use unbill_store_memory::InMemoryStore;

    #[tokio::test]
    async fn test_lock_guard_derefs_to_ledger_store() {
        let store = LockedStore::new(InMemoryStore::default());
        let guard = store.lock().await.unwrap();
        let ledgers = guard.list_ledgers().await.unwrap();
        assert!(ledgers.is_empty());
    }

    #[tokio::test]
    async fn test_lock_is_exclusive() {
        let store = LockedStore::new(InMemoryStore::default());
        let store2 = store.clone();

        let guard = store.lock().await.unwrap();

        let task = tokio::spawn(async move { store2.lock().await.unwrap() });

        // Yield so the spawned task gets a chance to attempt the lock.
        tokio::task::yield_now().await;
        assert!(
            !task.is_finished(),
            "second lock should block while first guard is held"
        );

        drop(guard);

        tokio::time::timeout(std::time::Duration::from_secs(1), task)
            .await
            .expect("lock should be released within timeout")
            .unwrap();
    }
}
