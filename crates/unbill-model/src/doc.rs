// LedgerDoc wraps an Automerge document and exposes typed operations.

use crate::error::UnbillError;
use crate::{
    AddBillOpError, AddDeviceOpError, AddUserOpError, BillId, Currency, Device, EffectiveBills,
    Ledger, LedgerId, LedgerMeta, NewBill, NewDevice, NewUser, NodeId, Timestamp, User,
};

use crate::ops;

type Result<T> = std::result::Result<T, UnbillError>;

/// A CRDT-backed in-memory ledger backed by a single Automerge document.
// sirno:witness:ledger-doc:begin
pub struct LedgerDoc {
    doc: automerge::AutoCommit,
}
// sirno:witness:ledger-doc:end

impl LedgerDoc {
    // sirno:witness:ledger-doc:begin
    /// Create and initialize a new ledger document.
    pub fn new(
        ledger_id: LedgerId,
        name: String,
        currency: Currency,
        created_at: Timestamp,
    ) -> Result<Self> {
        let mut doc = automerge::AutoCommit::new();
        ops::init_ledger(&mut doc, ledger_id, name, currency, created_at)?;
        Ok(Self { doc })
    }

    /// Load a ledger document from stored bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let doc = automerge::AutoCommit::load(bytes)
            .map_err(|e| UnbillError::Automerge(e.to_string()))?;
        Ok(Self { doc })
    }

    /// Create a blank document for use as the starting point of a sync-based
    /// load. The document has no ledger data until sync messages are applied
    /// via `receive_sync_message`.
    pub fn empty() -> Self {
        Self {
            doc: automerge::AutoCommit::new(),
        }
    }

    /// Returns `true` if no changes have been applied to this document.
    pub fn is_empty(&mut self) -> bool {
        self.doc.get_heads().is_empty()
    }

    /// Return the current heads of the document.
    pub fn heads(&mut self) -> Vec<automerge::ChangeHash> {
        self.doc.get_heads()
    }

    /// Return a SHA-256 fingerprint of the current CRDT state as 32 raw bytes.
    ///
    /// Hashes the concatenation of lexicographically sorted, raw 32-byte
    /// document heads. Like `heads`, this commits any pending transaction.
    /// The result is stable across save/load and converged replicas, and an
    /// empty document returns SHA-256 of the empty byte sequence.
    ///
    /// This includes change history: independently created documents with
    /// identical visible ledger values may have different fingerprints.
    /// Device-local metadata is excluded.
    pub fn state_hash(&mut self) -> [u8; 32] {
        use sha2::{Digest, Sha256};

        let mut heads = self.doc.get_heads();
        heads.sort_unstable();
        let mut hasher = Sha256::new();
        for head in heads {
            hasher.update(head.0);
        }
        hasher.finalize().into()
    }

    /// Serialize the full document to bytes for storage.
    pub fn save(&mut self) -> Vec<u8> {
        self.doc.save()
    }
    // sirno:witness:ledger-doc:end

    // --- read operations ---

    pub fn get_ledger(&self) -> Result<Ledger> {
        ops::get_ledger(&self.doc)
    }

    // sirno:witness:ledger-clone:begin
    /// Copy ledger content into a fresh history with only the cloning device authorized.
    pub fn clone_as(
        &self,
        ledger_id: LedgerId,
        device: NewDevice,
        now: Timestamp,
    ) -> Result<(Self, LedgerMeta)> {
        let mut ledger = self.get_ledger()?;
        const SUFFIX: &str = " (copy)";
        let name_limit = crate::validation::MAX_NAME_BYTES.saturating_sub(SUFFIX.len());
        ledger
            .name
            .truncate(ledger.name.floor_char_boundary(name_limit));
        ledger.name.push_str(SUFFIX);
        ledger.ledger_id = ledger_id;
        ledger.created_at = now;
        // A bill's author must be an authorized device. Keep bill IDs (the
        // rounding seeds) and amendment links, but never carry over peer access.
        for bill in &mut ledger.bills {
            bill.created_by_device = device.node_id.clone();
        }
        ledger.devices = vec![Device {
            node_id: device.node_id,
            label: device.label,
            added_at: now,
        }];
        let mut doc = Self::empty();
        autosurgeon::reconcile(&mut doc.doc, &ledger)
            .map_err(|error| UnbillError::Reconcile(error.to_string()))?;
        let meta = LedgerMeta {
            archived: false,
            ledger_id,
            name: ledger.name,
            currency: ledger.currency,
            created_at: now,
            updated_at: now,
        };
        Ok((doc, meta))
    }
    // sirno:witness:ledger-clone:end

    pub fn list_all_bills(&self) -> Result<Vec<crate::Bill>> {
        ops::list_all_bills(&self.doc)
    }

    pub fn list_bills(&self) -> Result<EffectiveBills> {
        ops::list_bills(&self.doc)
    }

    pub fn list_users(&self) -> Result<Vec<User>> {
        ops::list_users(&self.doc)
    }

    // --- write operations ---

    /// Change only the shared name, as a single Automerge scalar.
    pub fn rename(&mut self, name: &str) -> Result<()> {
        ops::rename_ledger(&mut self.doc, name)
    }

    pub fn add_bill(
        &mut self,
        input: NewBill,
        created_by_device: NodeId,
        now: Timestamp,
    ) -> std::result::Result<BillId, AddBillOpError> {
        self.add_bill_with_id(input, created_by_device, now, BillId::new())
    }

    /// Insert a bill using the ID allocated for its draft. Duplicate IDs are rejected.
    pub fn add_bill_with_id(
        &mut self,
        input: NewBill,
        created_by_device: NodeId,
        now: Timestamp,
        bill_id: BillId,
    ) -> std::result::Result<BillId, AddBillOpError> {
        ops::add_bill(&mut self.doc, input, created_by_device, now, bill_id)
    }

    pub fn add_user(
        &mut self,
        input: NewUser,
        now: Timestamp,
    ) -> std::result::Result<(), AddUserOpError> {
        ops::add_user(&mut self.doc, input, now)
    }

    pub fn set_user_archived(&mut self, user_id: &crate::UserId, archived: bool) -> Result<()> {
        ops::set_user_archived(&mut self.doc, user_id, archived)
    }

    pub fn add_device(
        &mut self,
        input: NewDevice,
        now: Timestamp,
    ) -> std::result::Result<(), AddDeviceOpError> {
        ops::add_device(&mut self.doc, input, now)
    }

    pub fn set_device_label(&mut self, node_id: &NodeId, label: crate::DeviceLabel) -> Result<()> {
        ops::set_device_label(&mut self.doc, node_id, label)
    }

    pub fn list_devices(&self) -> Result<Vec<Device>> {
        ops::list_devices(&self.doc)
    }

    // --- automerge sync ---

    // sirno:witness:sync-behavior:begin
    /// Merge all changes from `other` into this document.
    pub fn merge(&mut self, other: &mut LedgerDoc) -> Result<()> {
        self.doc
            .merge(&mut other.doc)
            .map_err(|e| UnbillError::Automerge(e.to_string()))?;
        Ok(())
    }

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
            .map_err(|e| UnbillError::Automerge(e.to_string()))?;
        Ok(())
    }
    // sirno:witness:sync-behavior:end

    /// Returns `true` if `node_id` is in `ledger.devices`.
    // sirno:witness:users-and-devices:begin
    pub fn is_device_authorized(&self, node_id: &NodeId) -> Result<bool> {
        let ledger = ops::get_ledger(&self.doc)?;
        Ok(ledger.devices.iter().any(|d| &d.node_id == node_id))
    }
    // sirno:witness:users-and-devices:end
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::UserId;

    fn ledger() -> LedgerDoc {
        LedgerDoc::new(
            LedgerId::from_u128(1),
            "Shared expenses".into(),
            Currency::from_code("USD").unwrap(),
            Timestamp::from_millis(1000),
        )
        .unwrap()
    }

    fn add_user(doc: &mut LedgerDoc, id: u128) {
        doc.add_user(
            NewUser {
                user_id: UserId::from_u128(id),
                display_name: format!("User {id}"),
            },
            Timestamp::from_millis(2000),
        )
        .unwrap();
    }

    #[test]
    fn rename_validates_and_concurrent_renames_converge() {
        let mut original = ledger();
        let before = original.state_hash();
        assert!(original.rename("  ").is_err());
        assert!(original.rename(&"💸".repeat(26)).is_err());
        original.rename("  Shared expenses  ").unwrap();
        assert_eq!(original.state_hash(), before);

        let bytes = original.save();
        let mut left = LedgerDoc::from_bytes(&bytes).unwrap();
        let mut right = LedgerDoc::from_bytes(&bytes).unwrap();
        let left_name = "💸".repeat(25);
        left.rename(&format!(" {left_name} ")).unwrap();
        right.rename("Updated on the right").unwrap();
        assert_eq!(left.get_ledger().unwrap().name, left_name);
        left.merge(&mut right).unwrap();
        right.merge(&mut left).unwrap();
        let winner = left.get_ledger().unwrap().name;
        assert!(winner == left_name || winner == "Updated on the right");
        assert_eq!(right.get_ledger().unwrap().name, winner);
        assert_eq!(left.state_hash(), right.state_hash());
    }

    #[test]
    fn rename_preserves_concurrent_updates_to_existing_users() {
        let mut renamed = ledger();
        add_user(&mut renamed, 1);
        let mut peer = LedgerDoc::from_bytes(&renamed.save()).unwrap();

        renamed.rename("Renamed").unwrap();
        assert_eq!(renamed.doc.get_last_local_change().unwrap().len(), 1);
        peer.set_user_archived(&UserId::from_u128(1), true).unwrap();
        renamed.merge(&mut peer).unwrap();
        peer.merge(&mut renamed).unwrap();

        for replica in [&renamed, &peer] {
            let ledger = replica.get_ledger().unwrap();
            assert_eq!(ledger.name, "Renamed");
            assert!(ledger.users.first().unwrap().archived);
        }
    }

    #[test]
    fn archiving_is_shared_reversible_and_preserved_by_other_mutations() {
        let mut doc = ledger();
        add_user(&mut doc, 1);
        assert!(!doc.list_users().unwrap().first().unwrap().archived);
        let before = doc.state_hash();
        let mut replica = LedgerDoc::from_bytes(&doc.save()).unwrap();
        doc.set_user_archived(&UserId::from_u128(1), true).unwrap();
        assert_ne!(before, doc.state_hash());
        add_user(&mut doc, 2);
        replica.merge(&mut doc).unwrap();
        assert!(replica.list_users().unwrap().first().unwrap().archived);
        let mut loaded = LedgerDoc::from_bytes(&replica.save()).unwrap();
        assert!(loaded.list_users().unwrap().first().unwrap().archived);
        let before = loaded.state_hash();
        assert!(
            loaded
                .set_user_archived(&UserId::from_u128(99), true)
                .is_err()
        );
        assert_eq!(before, loaded.state_hash());
        loaded
            .set_user_archived(&UserId::from_u128(1), false)
            .unwrap();
        assert!(!loaded.list_users().unwrap().first().unwrap().archived);
    }

    #[test]
    fn empty_state_hash_is_sha256_of_empty_bytes() {
        assert_eq!(
            LedgerDoc::empty().state_hash(),
            [
                0xe3, 0xb0, 0xc4, 0x42, 0x98, 0xfc, 0x1c, 0x14, 0x9a, 0xfb, 0xf4, 0xc8, 0x99, 0x6f,
                0xb9, 0x24, 0x27, 0xae, 0x41, 0xe4, 0x64, 0x9b, 0x93, 0x4c, 0xa4, 0x95, 0x99, 0x1b,
                0x78, 0x52, 0xb8, 0x55,
            ]
        );
    }

    #[test]
    fn state_hash_is_stable_across_reads_and_save_load() {
        let mut doc = ledger();
        add_user(&mut doc, 1);
        let hash = doc.state_hash();
        assert_eq!(doc.state_hash(), hash);
        let bytes = doc.save();
        let mut loaded = LedgerDoc::from_bytes(&bytes).unwrap();
        assert_eq!(loaded.state_hash(), hash);
        assert_eq!(doc.state_hash(), hash);
        assert_eq!(loaded.list_users().unwrap().len(), 1);
    }

    #[test]
    fn state_hash_changes_after_a_pending_write() {
        let mut doc = ledger();
        let before = doc.state_hash();
        add_user(&mut doc, 1);
        assert_ne!(doc.state_hash(), before);
    }

    #[test]
    fn state_hash_converges_after_concurrent_changes_in_any_merge_order() {
        let mut base = ledger();
        let bytes = base.save();
        let mut left = LedgerDoc::from_bytes(&bytes).unwrap();
        let mut right = LedgerDoc::from_bytes(&bytes).unwrap();
        add_user(&mut left, 1);
        add_user(&mut right, 2);
        assert_ne!(left.state_hash(), right.state_hash());

        let mut forward = LedgerDoc::from_bytes(&bytes).unwrap();
        let mut reverse = LedgerDoc::from_bytes(&bytes).unwrap();
        forward.merge(&mut left).unwrap();
        forward.merge(&mut right).unwrap();
        reverse.merge(&mut right).unwrap();
        reverse.merge(&mut left).unwrap();
        assert_eq!(forward.heads().len(), 2);
        assert_eq!(forward.state_hash(), reverse.state_hash());
        let merged_hash = forward.state_hash();
        assert_eq!(
            LedgerDoc::from_bytes(&forward.save()).unwrap().state_hash(),
            merged_hash
        );

        left.merge(&mut right).unwrap();
        right.merge(&mut left).unwrap();
        assert_eq!(left.state_hash(), merged_hash);
        assert_eq!(right.state_hash(), merged_hash);
    }
}

#[cfg(test)]
mod device_label_tests {
    use crate::{
        Currency, DeviceLabel, LedgerDoc, LedgerId, NewDevice, NewUser, NodeId, Timestamp,
        UnbillError, UserId,
    };
    fn doc() -> LedgerDoc {
        let mut doc = LedgerDoc::new(
            LedgerId::from_u128(1),
            "Trip".into(),
            Currency::from_code("USD").unwrap(),
            Timestamp::from_millis(1),
        )
        .unwrap();
        doc.add_device(
            NewDevice {
                node_id: NodeId::new("peer".into()),
                label: DeviceLabel::new("Phone".into()).unwrap(),
            },
            Timestamp::from_millis(2),
        )
        .unwrap();
        doc
    }
    #[test]
    fn names_are_validated_at_wire_and_document_boundaries() {
        assert!(DeviceLabel::new("  ".into()).is_err());
        assert!(DeviceLabel::new("x".repeat(101)).is_err());
        assert!(serde_json::from_str::<DeviceLabel>(r#""  ""#).is_err());
        assert_eq!(
            DeviceLabel::new("  Phone  ".into()).unwrap().as_str(),
            "Phone"
        );
    }
    #[test]
    fn rename_changes_hash_and_survives_other_mutations_roundtrip_and_merge() {
        let mut local = doc();
        let mut peer = LedgerDoc::from_bytes(&local.save()).unwrap();
        let before = local.state_hash();
        local
            .set_device_label(
                &NodeId::new("peer".into()),
                DeviceLabel::new("Kitchen iPad".into()).unwrap(),
            )
            .unwrap();
        assert_ne!(local.state_hash(), before);
        local
            .add_user(
                NewUser {
                    user_id: UserId::from_u128(2),
                    display_name: "Alice".into(),
                },
                Timestamp::from_millis(3),
            )
            .unwrap();
        peer.merge(&mut local).unwrap();
        let restored = LedgerDoc::from_bytes(&peer.save()).unwrap();
        assert_eq!(
            restored.list_devices().unwrap()[0].label.as_str(),
            "Kitchen iPad"
        );
        assert_eq!(restored.list_users().unwrap().len(), 1);
    }
    #[test]
    fn rename_is_ledger_scoped_and_cannot_authorize_a_new_device() {
        let mut first = doc();
        let second = doc();
        first
            .set_device_label(
                &NodeId::new("peer".into()),
                DeviceLabel::new("Laptop".into()).unwrap(),
            )
            .unwrap();
        assert_eq!(second.list_devices().unwrap()[0].label.as_str(), "Phone");
        let before = first.state_hash();
        assert!(matches!(
            first.set_device_label(
                &NodeId::new("stranger".into()),
                DeviceLabel::new("Unknown".into()).unwrap()
            ),
            Err(UnbillError::DeviceNotFound(_))
        ));
        assert_eq!(first.state_hash(), before);
        assert_eq!(first.list_devices().unwrap().len(), 1);
    }

    #[test]
    fn documents_reject_missing_or_invalid_device_names() {
        use automerge::{ReadDoc, transaction::Transactable};
        for invalid in [None, Some("  ")] {
            let mut original = doc();
            let mut raw = automerge::AutoCommit::load(&original.save()).unwrap();
            let devices = raw.get(automerge::ROOT, "devices").unwrap().unwrap().1;
            let device = raw.get(&devices, 0).unwrap().unwrap().1;
            if let Some(name) = invalid {
                raw.put(&device, "label", name).unwrap();
            } else {
                raw.delete(&device, "label").unwrap();
            }
            let loaded = LedgerDoc::from_bytes(&raw.save()).unwrap();
            assert!(loaded.list_devices().is_err());
        }
    }
}
