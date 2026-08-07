import Foundation

// The bridge boundary. Today a mock backs it; later a Rust-backed implementation
// (UnbillConsole via UniFFI/swift-bridge) slots in behind this same protocol
// without any UI changes. Async to match the console's async orchestration API.
// sirno:witness:unbill-apple:begin
protocol ConsoleClient {
    func ledgers() async throws -> [LedgerSummary]
    func ledgerDetail(id: String) async throws -> LedgerDetail
    @discardableResult
    func createLedger(name: String, currency: String) async throws -> LedgerSummary
    // Create a new named user in a ledger.
    @discardableResult
    func createUser(ledgerID: String, displayName: String) async throws -> User
    // All users known on this device (across ledgers) — for importing.
    func knownUsers() async throws -> [User]
    // Add an already-known user (by id) to a ledger.
    @discardableResult
    func addUser(ledgerID: String, userID: String) async throws -> User
    // Save a bill split equally among the given payer(s) and payee(s).
    func saveBill(
        ledgerID: String,
        description: String,
        amountCents: Int64,
        payerUserIDs: [String],
        payeeUserIDs: [String]
    ) async throws
    // Resolve a conflict group: keep `selectedBillID`, supersede all `conflictingBillIDs`.
    func resolveConflict(
        ledgerID: String,
        selectedBillID: String,
        conflictingBillIDs: [String]
    ) async throws

    // This device's node id.
    func deviceID() async throws -> String
    // Create an invitation URL others can use to join this ledger.
    func createInvitation(ledgerID: String) async throws -> String
    // Join a ledger from an invitation URL.
    func joinLedger(url: String, label: String?) async throws
    // Peer devices known to this device.
    func syncDevices() async throws -> [SyncDevice]
    // Run one sync round with a peer.
    func syncOnce(peerNodeID: String) async throws
}
// sirno:witness:unbill-apple:end
