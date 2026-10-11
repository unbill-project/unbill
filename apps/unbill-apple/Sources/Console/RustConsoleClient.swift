import Foundation

// ConsoleClient backed by the real Rust core (unbill-ffi via UniFFI).
//
// An `actor` so its calls run OFF the main thread: the FFI methods are
// mostly synchronous (they block_on a tokio runtime in Rust), and blocking the main
// thread would freeze the UI (e.g. a navigation transition stalling on the old
// title). Isolating to the actor's executor keeps SwiftUI responsive. Moving to
// async UniFFI later removes the blocking entirely. Peer sync already uses an
// async UniFFI call so network waits yield this actor to local ledger queries.
//
// Maps the aggregated FFI DTOs (bootstrap / ledger detail) to the app's models.
actor RustConsoleClient: ConsoleClient {
    private let console: FfiConsole

    init() throws {
        #if targetEnvironment(macCatalyst) || os(macOS)
        console = try FfiConsole.open(dir: defaultDataDirectory())
        #else
        let base = FileManager.default
            .urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
        let dir = base.appendingPathComponent("unbill", isDirectory: true)
        console = try FfiConsole.open(dir: dir.path)
        #endif
    }

    func serviceBuildInfo() async throws -> FfiBuildInfo {
        try console.serviceBuildInfo()
    }

    func supportedCurrencies() async -> [String] {
        supportedCurrencyCodes()
    }

    func ledgers() async throws -> [LedgerSummary] {
        try console.bootstrap().ledgers.map(Self.summary).sorted {
            if $0.latestBillAtMs != $1.latestBillAtMs {
                return ($0.latestBillAtMs ?? Int64.min) > ($1.latestBillAtMs ?? Int64.min)
            }
            if $0.name != $1.name { return $0.name < $1.name }
            return $0.ledgerID < $1.ledgerID
        }
    }

    @discardableResult
    func createLedger(name: String, currency: String) async throws -> LedgerSummary {
        Self.summary(try console.createLedger(name: name, currency: currency))
    }

    @discardableResult
    func setLedgerArchived(ledgerID: String, archived: Bool) async throws {
        try console.setLedgerArchived(ledgerId: ledgerID, archived: archived)
    }

    func setUserArchived(ledgerID: String, userID: String, archived: Bool) async throws {
        try console.setUserArchived(ledgerId: ledgerID, userId: userID, archived: archived)
    }

    func createUser(ledgerID: String, displayName: String) async throws -> User {
        Self.user(try console.createUser(ledgerId: ledgerID, displayName: displayName))
    }

    func knownUsers() async throws -> [User] {
        try console.bootstrap().allUsers.map(Self.user)
    }

    @discardableResult
    func addUser(ledgerID: String, userID: String) async throws -> User {
        Self.user(try console.addUser(ledgerId: ledgerID, userId: userID))
    }

    func previewBill(amountCents: Int64, payers: [BillShareInput], payees: [BillShareInput], previewID: String) async throws -> BillSplitPreview {
        let split = try console.previewBillSplit(amountCents: amountCents,
            payers: payers.map { FfiShareInput(userId: $0.userID, shares: $0.shares) },
            payees: payees.map { FfiShareInput(userId: $0.userID, shares: $0.shares) }, billId: previewID)
        return BillSplitPreview(
            payers: Dictionary(split.payerAmounts.map { ($0.userId, $0.amountCents) }, uniquingKeysWith: +),
            payees: Dictionary(split.payeeAmounts.map { ($0.userId, $0.amountCents) }, uniquingKeysWith: +))
    }

    func saveBill(ledgerID: String, description: String, amountCents: Int64, payers: [BillShareInput], payees: [BillShareInput], previousBillIDs: [String], billID: String) async throws {
        _ = try console.saveBill(ledgerId: ledgerID, amountCents: amountCents, description: description,
            payers: payers.map { FfiShareInput(userId: $0.userID, shares: $0.shares) },
            payees: payees.map { FfiShareInput(userId: $0.userID, shares: $0.shares) }, draft: FfiBillDraft(billId: billID, prevBillIds: previousBillIDs))
    }

    func ledgerDetail(id: String) async throws -> LedgerDetail {
        let d = try console.ledgerDetail(ledgerId: id)
        return LedgerDetail(
            summary: Self.summary(d.summary),
            emojiFingerprint: d.emojiFingerprint,
            users: d.users.map(Self.user),
            devices: d.devices.map { SyncDevice(nodeID: $0.nodeId, label: $0.label, ledgerNames: $0.ledgerNames) },
            bills: d.bills.map(Self.bill),
            conflicts: d.conflicts.map {
                ConflictGroup(conflicting: $0.conflicting.map(Self.bill), ancestors: $0.ancestors.map(Self.bill))
            },
            settlement: d.settlement.map {
                Transaction(fromName: $0.fromName, toName: $0.toName, amountCents: $0.amountCents)
            }
        )
    }

    func ledgerUpdates() async -> AsyncStream<String?> {
        AsyncStream { continuation in
            let observer = LedgerUpdateObserver(continuation: continuation)
            let subscription = console.observe(observer: observer)
            continuation.onTermination = { _ in subscription.cancel() }
        }
    }

    func resolveConflict(
        ledgerID: String,
        selectedBillID: String,
        conflictingBillIDs: [String]
    ) async throws {
        _ = try console.resolveConflict(
            ledgerId: ledgerID,
            selectedBillId: selectedBillID,
            conflictingBillIds: conflictingBillIDs
        )
    }

    func deviceID() async throws -> String {
        console.deviceId()
    }

    func createInvitation(ledgerID: String) async throws -> String {
        try console.createInvitation(ledgerId: ledgerID)
    }

    func joinLedger(url: String, label: String) async throws {
        try console.joinLedger(url: url, label: label)
    }

    func syncDevices() async throws -> [SyncDevice] {
        try console.bootstrap().devices.map {
            SyncDevice(nodeID: $0.nodeId, label: $0.label, ledgerNames: $0.ledgerNames)
        }
    }

    func syncOnce(peerNodeID: String) async throws {
        try await console.syncOnce(peerNodeId: peerNodeID)
    }

    // MARK: - Mapping (nonisolated: pure value transforms)

    private static func summary(_ s: FfiLedgerSummary) -> LedgerSummary {
        LedgerSummary(
            archived: s.archived,
            ledgerID: s.ledgerId, name: s.name, currency: s.currency,
            createdAtMs: s.createdAtMs, updatedAtMs: s.updatedAtMs,
            userCount: Int(s.userCount), userNames: s.userNames,
            latestBillAtMs: s.latestBillAtMs
        )
    }

    private static func bill(_ b: FfiBill) -> Bill {
        Bill(
            id: b.id, amountCents: b.amountCents, description: b.description,
            createdAtMs: b.createdAtMs,
            payers: b.payers.map(share), payees: b.payees.map(share)
        )
    }

    private static func share(_ s: FfiShare) -> Share {
        Share(userID: s.userId, shares: UInt32(s.shares), displayName: s.displayName)
    }

    private static func user(_ u: FfiUser) -> User {
        User(userID: u.userId, displayName: u.displayName, archived: u.archived, addedAtMs: u.addedAtMs)
    }
}

private final class LedgerUpdateObserver: FfiConsoleObserver {
    let continuation: AsyncStream<String?>.Continuation

    init(continuation: AsyncStream<String?>.Continuation) {
        self.continuation = continuation
    }

    func onEvent(event: FfiServiceEvent) {
        switch event {
        case .ledgerUpdated(let ledgerID):
            continuation.yield(ledgerID)
        case .resyncNeeded:
            continuation.yield(nil)
        default:
            break
        }
    }
}
