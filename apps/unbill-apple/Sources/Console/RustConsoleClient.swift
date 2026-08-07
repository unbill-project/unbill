import Foundation

// ConsoleClient backed by the real Rust core (unbill-ffi via UniFFI).
//
// An `actor` so its calls run OFF the main thread: the FFI methods are
// synchronous (they block_on a tokio runtime in Rust), and blocking the main
// thread would freeze the UI (e.g. a navigation transition stalling on the old
// title). Isolating to the actor's executor keeps SwiftUI responsive. Moving to
// async UniFFI later removes the blocking entirely.
//
// Maps the aggregated FFI DTOs (bootstrap / ledger detail) to the app's models.
// On first run it seeds one ledger with users and bills so the detail screen
// shows real, Rust-computed settlement. Persisted to disk via FsStore.
actor RustConsoleClient: ConsoleClient {
    private let console: FfiConsole

    init() throws {
        let base = FileManager.default
            .urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
        let dir = base.appendingPathComponent("unbill", isDirectory: true)
        console = try FfiConsole.open(dir: dir.path)
    }

    func ledgers() async throws -> [LedgerSummary] {
        var boot = try console.bootstrap()
        if boot.ledgers.isEmpty {
            try seed()
            boot = try console.bootstrap()
        }
        return boot.ledgers.map(Self.summary)
    }

    @discardableResult
    func createLedger(name: String, currency: String) async throws -> LedgerSummary {
        Self.summary(try console.createLedger(name: name, currency: currency))
    }

    @discardableResult
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

    func saveBill(
        ledgerID: String,
        description: String,
        amountCents: Int64,
        payerUserIDs: [String],
        payeeUserIDs: [String]
    ) async throws {
        _ = try console.saveBill(
            ledgerId: ledgerID,
            amountCents: amountCents,
            description: description,
            payers: payerUserIDs.map { FfiShareInput(userId: $0, shares: 1) },
            payees: payeeUserIDs.map { FfiShareInput(userId: $0, shares: 1) },
            prevBillIds: []
        )
    }

    func ledgerDetail(id: String) async throws -> LedgerDetail {
        let d = try console.ledgerDetail(ledgerId: id)
        return LedgerDetail(
            summary: Self.summary(d.summary),
            users: d.users.map(Self.user),
            bills: d.bills.map(Self.bill),
            conflicts: d.conflicts.map {
                ConflictGroup(conflicting: $0.conflicting.map(Self.bill), ancestors: $0.ancestors.map(Self.bill))
            },
            settlement: d.settlement.map {
                Transaction(fromName: $0.fromName, toName: $0.toName, amountCents: $0.amountCents)
            }
        )
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

    func joinLedger(url: String, label: String?) async throws {
        try console.joinLedger(url: url, label: label)
    }

    func syncDevices() async throws -> [SyncDevice] {
        try console.bootstrap().devices.map {
            SyncDevice(nodeID: $0.nodeId, label: $0.label, ledgerNames: $0.ledgerNames)
        }
    }

    func syncOnce(peerNodeID: String) async throws {
        try console.syncOnce(peerNodeId: peerNodeID)
    }

    // MARK: - Seed (first run only)

    private func seed() throws {
        // A shared trip with a real split, so settlement is non-trivial.
        let trip = try console.createLedger(name: "Iceland Trip", currency: "USD")
        let alice = try console.createUser(ledgerId: trip.ledgerId, displayName: "Alice")
        let bob = try console.createUser(ledgerId: trip.ledgerId, displayName: "Bob")
        let carol = try console.createUser(ledgerId: trip.ledgerId, displayName: "Carol")

        // Alice paid $120 groceries, split three ways.
        _ = try console.saveBill(
            ledgerId: trip.ledgerId, amountCents: 12_000, description: "Groceries",
            payers: [FfiShareInput(userId: alice.userId, shares: 1)],
            payees: [alice, bob, carol].map { FfiShareInput(userId: $0.userId, shares: 1) },
            prevBillIds: []
        )
        // Bob paid $66 gas, split between Alice and Bob.
        _ = try console.saveBill(
            ledgerId: trip.ledgerId, amountCents: 6_600, description: "Gas",
            payers: [FfiShareInput(userId: bob.userId, shares: 1)],
            payees: [alice, bob].map { FfiShareInput(userId: $0.userId, shares: 1) },
            prevBillIds: []
        )

        // A second, empty ledger.
        _ = try console.createLedger(name: "Flat 4B", currency: "EUR")

        // A ledger with a conflict to resolve: two competing amendments of the
        // same original bill (both supersede it, neither supersedes the other).
        let split = try console.createLedger(name: "Split Disagreement", currency: "USD")
        let dave = try console.createUser(ledgerId: split.ledgerId, displayName: "Dave")
        let erin = try console.createUser(ledgerId: split.ledgerId, displayName: "Erin")
        let dinner = try console.saveBill(
            ledgerId: split.ledgerId, amountCents: 5_000, description: "Dinner",
            payers: [FfiShareInput(userId: dave.userId, shares: 1)],
            payees: [dave, erin].map { FfiShareInput(userId: $0.userId, shares: 1) },
            prevBillIds: []
        )
        _ = try console.saveBill(
            ledgerId: split.ledgerId, amountCents: 5_000, description: "Dinner — split evenly",
            payers: [FfiShareInput(userId: dave.userId, shares: 1)],
            payees: [dave, erin].map { FfiShareInput(userId: $0.userId, shares: 1) },
            prevBillIds: [dinner]
        )
        _ = try console.saveBill(
            ledgerId: split.ledgerId, amountCents: 5_000, description: "Dinner — Erin's treat",
            payers: [FfiShareInput(userId: erin.userId, shares: 1)],
            payees: [dave, erin].map { FfiShareInput(userId: $0.userId, shares: 1) },
            prevBillIds: [dinner]
        )
    }

    // MARK: - Mapping (nonisolated: pure value transforms)

    private static func summary(_ s: FfiLedgerSummary) -> LedgerSummary {
        LedgerSummary(
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
        User(userID: u.userId, displayName: u.displayName, addedAtMs: u.addedAtMs)
    }
}
