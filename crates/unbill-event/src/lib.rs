// sirno:witness:unbill-event:begin
#[derive(Clone, Debug)]
pub enum ServiceEvent {
    DeviceIdentityInitialized,
    DeviceLabelsUpdated,
    PendingInvitationsUpdated,
    LedgerUpdated {
        ledger_id: String,
    },
    PeerConnected {
        ledger_id: String,
        peer: String,
    },
    PeerDisconnected {
        ledger_id: String,
        peer: String,
    },
    SyncError {
        ledger_id: String,
        peer: String,
        error: String,
    },
}
// sirno:witness:unbill-event:end
