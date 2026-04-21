// LedgerDoc wraps an Automerge document and exposes typed operations.

use tokio::sync::broadcast;

use crate::error::Result;
use crate::model::{
    Currency, Device, EffectiveBills, Ledger, NewBill, NewDevice, NewUser, NodeId, Timestamp, Ulid,
    User,
};

use super::ops;

/// A CRDT-backed in-memory ledger backed by a single Automerge document.
pub struct LedgerDoc {
    doc: automerge::AutoCommit,
    pub changes: broadcast::Sender<ChangeEvent>,
}

#[derive(Clone, Debug)]
pub enum ChangeEvent {
    LocalWrite,
    RemoteApplied,
}

impl LedgerDoc {
    /// Create and initialize a new ledger document.
    pub fn new(
        ledger_id: Ulid,
        name: String,
        currency: Currency,
        created_at: Timestamp,
    ) -> Result<Self> {
        let mut doc = automerge::AutoCommit::new();
        ops::init_ledger(&mut doc, ledger_id, name, currency, created_at)?;
        let (tx, _) = broadcast::channel(64);
        Ok(Self { doc, changes: tx })
    }

    /// Load a ledger document from stored bytes.
    pub fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        let doc = automerge::AutoCommit::load(bytes)?;
        let (tx, _) = broadcast::channel(64);
        Ok(Self { doc, changes: tx })
    }

    /// Serialize the full document to bytes for storage.
    pub fn save(&mut self) -> Vec<u8> {
        self.doc.save()
    }

    // --- read operations ---

    pub fn get_ledger(&self) -> Result<Ledger> {
        ops::get_ledger(&self.doc)
    }

    pub fn list_all_bills(&self) -> Result<Vec<crate::model::Bill>> {
        ops::list_all_bills(&self.doc)
    }

    pub fn list_bills(&self) -> Result<EffectiveBills> {
        ops::list_bills(&self.doc)
    }

    pub fn list_users(&self) -> Result<Vec<User>> {
        ops::list_users(&self.doc)
    }

    // --- write operations ---

    pub fn add_bill(
        &mut self,
        input: NewBill,
        created_by_device: NodeId,
        now: Timestamp,
    ) -> Result<Ulid> {
        let id = ops::add_bill(&mut self.doc, input, created_by_device, now)?;
        let _ = self.changes.send(ChangeEvent::LocalWrite);
        Ok(id)
    }

    pub fn add_user(&mut self, input: NewUser, now: Timestamp) -> Result<()> {
        ops::add_user(&mut self.doc, input, now)?;
        let _ = self.changes.send(ChangeEvent::LocalWrite);
        Ok(())
    }

    pub fn add_device(&mut self, input: NewDevice, now: Timestamp) -> Result<()> {
        ops::add_device(&mut self.doc, input, now)?;
        let _ = self.changes.send(ChangeEvent::LocalWrite);
        Ok(())
    }

    pub fn list_devices(&self) -> Result<Vec<Device>> {
        ops::list_devices(&self.doc)
    }

    // --- automerge sync ---

    pub fn generate_sync_message(
        &mut self,
        sync_state: &mut automerge::sync::State,
    ) -> Option<automerge::sync::Message> {
        use automerge::sync::SyncDoc as _;
        self.doc.sync().generate_sync_message(sync_state)
    }

    pub fn receive_sync_message(
        &mut self,
        sync_state: &mut automerge::sync::State,
        msg: automerge::sync::Message,
    ) -> Result<()> {
        use automerge::sync::SyncDoc as _;
        self.doc
            .sync()
            .receive_sync_message(sync_state, msg)
            .map_err(|e| crate::error::UnbillError::Other(e.into()))?;
        let _ = self.changes.send(ChangeEvent::RemoteApplied);
        Ok(())
    }

    /// Returns `true` if `node_id` is in `ledger.devices`.
    pub fn is_device_authorized(&self, node_id: &NodeId) -> Result<bool> {
        let ledger = ops::get_ledger(&self.doc)?;
        Ok(ledger.devices.iter().any(|d| &d.node_id == node_id))
    }
}
