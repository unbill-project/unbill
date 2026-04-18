// LedgerDoc wraps an Automerge document and exposes typed operations.

use tokio::sync::broadcast;

use crate::error::Result;
use crate::model::{
    BillAmendment, Currency, Device, EffectiveBill, Ledger, Member, NewBill, NewDevice, NewMember,
    NodeId, Timestamp, Ulid,
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

    pub fn list_bills(&self) -> Result<Vec<EffectiveBill>> {
        ops::list_bills(&self.doc)
    }

    pub fn list_members(&self) -> Result<Vec<Member>> {
        ops::list_members(&self.doc)
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

    pub fn amend_bill(
        &mut self,
        bill_id: &Ulid,
        input: BillAmendment,
        now: Timestamp,
    ) -> Result<()> {
        ops::amend_bill(&mut self.doc, bill_id, input, now)?;
        let _ = self.changes.send(ChangeEvent::LocalWrite);
        Ok(())
    }

    pub fn delete_bill(&mut self, bill_id: &Ulid) -> Result<()> {
        ops::delete_bill(&mut self.doc, bill_id)?;
        let _ = self.changes.send(ChangeEvent::LocalWrite);
        Ok(())
    }

    pub fn restore_bill(&mut self, bill_id: &Ulid) -> Result<()> {
        ops::restore_bill(&mut self.doc, bill_id)?;
        let _ = self.changes.send(ChangeEvent::LocalWrite);
        Ok(())
    }

    pub fn add_member(&mut self, input: NewMember, now: Timestamp) -> Result<()> {
        ops::add_member(&mut self.doc, input, now)?;
        let _ = self.changes.send(ChangeEvent::LocalWrite);
        Ok(())
    }

    pub fn remove_member(&mut self, user_id: &Ulid) -> Result<()> {
        ops::remove_member(&mut self.doc, user_id)?;
        let _ = self.changes.send(ChangeEvent::LocalWrite);
        Ok(())
    }

    pub fn add_device(&mut self, input: NewDevice, now: Timestamp) -> Result<()> {
        ops::add_device(&mut self.doc, input, now)?;
        let _ = self.changes.send(ChangeEvent::LocalWrite);
        Ok(())
    }

    pub fn remove_device(&mut self, node_id: &NodeId) -> Result<()> {
        ops::remove_device(&mut self.doc, node_id)?;
        let _ = self.changes.send(ChangeEvent::LocalWrite);
        Ok(())
    }

    pub fn list_devices(&self) -> Result<Vec<Device>> {
        ops::list_devices(&self.doc)
    }
}
