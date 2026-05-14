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

impl<T: LedgerStore + Default + 'static> Default for LockedStore<T> {
    fn default() -> Self {
        Self::new(T::default())
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
