// Helpers for reading and writing service-level metadata blobs via the store.
// These are JSON-encoded values stored under well-known device metadata keys.

use std::collections::HashMap;

use unbill_model::{Invitation, StorageError, UnbillError};

use crate::LedgerStore;

type Result<T> = std::result::Result<T, UnbillError>;

const DEVICE_LABELS_KEY: &str = "device_labels.json";
const PENDING_INVITATIONS_KEY: &str = "pending_invitations.json";

// sirno:witness:shared-and-local-state:begin
pub async fn load_device_labels(store: &dyn LedgerStore) -> Result<HashMap<String, String>> {
    match store.load_device_meta(DEVICE_LABELS_KEY).await? {
        None => Ok(HashMap::new()),
        Some(bytes) => serde_json::from_slice(&bytes).map_err(|e| {
            UnbillError::Storage(StorageError::Serialization(format!(
                "device_labels.json: {e}"
            )))
        }),
    }
}

pub async fn save_device_labels(
    store: &dyn LedgerStore,
    labels: &HashMap<String, String>,
) -> Result<()> {
    let bytes = serde_json::to_vec(labels).map_err(|e| {
        UnbillError::Storage(StorageError::Serialization(format!(
            "serialize device_labels: {e}"
        )))
    })?;
    store.save_device_meta(DEVICE_LABELS_KEY, &bytes).await?;
    Ok(())
}

pub async fn load_pending_invitations(
    store: &dyn LedgerStore,
) -> Result<HashMap<String, Invitation>> {
    match store.load_device_meta(PENDING_INVITATIONS_KEY).await? {
        None => Ok(HashMap::new()),
        Some(bytes) => serde_json::from_slice(&bytes).map_err(|e| {
            UnbillError::Storage(StorageError::Serialization(format!(
                "pending_invitations.json: {e}"
            )))
        }),
    }
}

pub async fn save_pending_invitations(
    store: &dyn LedgerStore,
    map: &HashMap<String, Invitation>,
) -> Result<()> {
    let bytes = serde_json::to_vec(map).map_err(|e| {
        UnbillError::Storage(StorageError::Serialization(format!(
            "serialize pending_invitations: {e}"
        )))
    })?;
    store
        .save_device_meta(PENDING_INVITATIONS_KEY, &bytes)
        .await?;
    Ok(())
}
// sirno:witness:shared-and-local-state:end
