import Foundation

// Plain Swift value types mirroring the Rust DTOs projected by `UnbillConsole`
// (see apps/unbill-ui-native/src/api.rs). These are intentionally UI-facing and
// will map 1:1 to the eventual FFI-generated types behind `ConsoleClient`.

struct LedgerSummary: Identifiable, Hashable {
    let ledgerID: String
    let name: String
    let currency: String
    let createdAtMs: Int64
    let updatedAtMs: Int64
    let userCount: Int
    let userNames: [String]
    let latestBillAtMs: Int64?

    var id: String { ledgerID }
}

struct User: Identifiable, Hashable {
    let userID: String
    let displayName: String
    let addedAtMs: Int64

    var id: String { userID }
}

// A peer device known to this device (across ledgers).
struct SyncDevice: Identifiable, Hashable {
    let nodeID: String
    let label: String
    let ledgerNames: [String]

    var id: String { nodeID }
}

struct Share: Hashable {
    let userID: String
    let shares: UInt32
    let displayName: String
}

struct Bill: Identifiable, Hashable {
    let id: String
    let amountCents: Int64
    let description: String
    let createdAtMs: Int64
    let payers: [Share]
    let payees: [Share]
}

struct Transaction: Hashable {
    let fromName: String
    let toName: String
    let amountCents: Int64
}

// Competing amendments of a common ancestor bill. Resolving picks one
// `conflicting` version to keep and supersedes the rest.
struct ConflictGroup: Identifiable, Hashable {
    let conflicting: [Bill]
    let ancestors: [Bill]

    // Stable identity from the set of competing bill ids.
    var id: String { conflicting.map(\.id).sorted().joined(separator: "|") }
}

struct LedgerDetail: Hashable {
    let summary: LedgerSummary
    let users: [User]
    let bills: [Bill]
    let conflicts: [ConflictGroup]
    let settlement: [Transaction]
}
