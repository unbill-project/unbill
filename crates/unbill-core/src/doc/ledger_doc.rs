// LedgerDoc wraps an Automerge document and exposes typed operations.

use tokio::sync::broadcast;

use crate::error::Result;
use crate::model::{BillAmendment, Currency, EffectiveBill, Ledger, Member, NewBill, NodeId, Timestamp, Ulid};

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

    /// Load a ledger document from saved bytes (snapshot + optional incrementals).
    pub fn from_bytes(snapshot: &[u8], incrementals: &[Vec<u8>]) -> anyhow::Result<Self> {
        let mut doc = if snapshot.is_empty() {
            automerge::AutoCommit::new()
        } else {
            automerge::AutoCommit::load(snapshot)?
        };
        for chunk in incrementals {
            doc.load_incremental(chunk)?;
        }
        let (tx, _) = broadcast::channel(64);
        Ok(Self { doc, changes: tx })
    }

    /// Full snapshot bytes (for compaction / initial save).
    pub fn save(&mut self) -> Vec<u8> {
        self.doc.save()
    }

    /// Incremental bytes for changes since the last call to `save` or
    /// `save_incremental`. Returns empty if there are no new changes.
    pub fn save_incremental(&mut self) -> Vec<u8> {
        self.doc.save_incremental()
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

    pub fn add_bill(&mut self, input: NewBill, created_by_device: NodeId, now: Timestamp) -> Result<Ulid> {
        let id = ops::add_bill(&mut self.doc, input, created_by_device, now)?;
        let _ = self.changes.send(ChangeEvent::LocalWrite);
        Ok(id)
    }

    pub fn amend_bill(&mut self, bill_id: &Ulid, input: BillAmendment, now: Timestamp) -> Result<()> {
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
}
