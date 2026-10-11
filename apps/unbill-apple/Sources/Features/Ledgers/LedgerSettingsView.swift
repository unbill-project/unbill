import SwiftUI
import UIKit

// sirno:witness:unbill-apple:begin
struct LedgerSettingsView: View {
    let console: ConsoleClient
    let detail: LedgerDetail
    @Environment(\.dismiss) private var dismiss
    @State private var copied = false
    @State private var syncingNodeID: String?
    @State private var lastSyncedAt: [String: Date] = [:]
    @State private var syncErrors: [String: String] = [:]

    @State private var archiving = false
    @State private var archiveError: String?

    private var busy: Bool { syncingNodeID != nil || archiving }

    var body: some View {
        NavigationStack {
            List {
                Section(detail.summary.name) {
                    VStack(alignment: .leading, spacing: 12) {
                        Text("Ledger fingerprint")
                            .font(.headline)
                        Text(detail.emojiFingerprint)
                            .font(.title2)
                            .fixedSize(horizontal: false, vertical: true)
                            .textSelection(.enabled)
                        Text("Matching fingerprints suggest these devices have the same ledger state.")
                            .font(.caption)
                            .foregroundStyle(.secondary)
                        Button(copied ? "Fingerprint copied" : "Copy fingerprint", systemImage: "doc.on.doc") {
                            UIPasteboard.general.string = detail.emojiFingerprint
                            copied = true
                        }
                    }
                    .padding(.vertical, 4)
                }
                Section("Ledger Devices") {
                    if detail.devices.isEmpty { Text("No peer devices").foregroundStyle(.secondary) }
                    ForEach(detail.devices) { peer in
                        PeerDeviceRow(
                            peer: peer,
                            syncing: syncingNodeID == peer.nodeID,
                            disabled: busy,
                            lastSyncedAt: lastSyncedAt[peer.nodeID],
                            syncError: syncErrors[peer.nodeID]
                        ) {
                            Task { await sync(peer.nodeID) }
                        }
                    }
                }
                Section {
                    Button(detail.summary.archived ? "Restore Ledger" : "Archive Ledger") {
                        Task {
                            guard !busy else { return }
                            archiving = true
                            defer { archiving = false }
                            do {
                                try await console.setLedgerArchived(ledgerID: detail.summary.ledgerID, archived: !detail.summary.archived)
                                dismiss()
                            } catch { archiveError = error.localizedDescription }
                        }
                    }
                    .disabled(busy)
                    if let archiveError { Text(archiveError).foregroundStyle(.red) }
                } footer: {
                    Text("Archiving hides this ledger on this device. Its data and syncing are retained.")
                }
            }
            .navigationTitle("Ledger Settings")
            .toolbar {
                ToolbarItem(placement: .confirmationAction) {
                    Button("Done") { dismiss() }
                        .disabled(busy)
                }
            }
        }
        .interactiveDismissDisabled(busy)
        .onChange(of: detail.emojiFingerprint) { _, _ in copied = false }
    }

    private func sync(_ nodeID: String) async {
        guard !busy else { return }
        syncingNodeID = nodeID
        syncErrors[nodeID] = nil
        defer { syncingNodeID = nil }
        do {
            try await console.syncOnce(peerNodeID: nodeID)
            lastSyncedAt[nodeID] = Date()
        } catch {
            syncErrors[nodeID] = error.localizedDescription
        }
    }

}
// sirno:witness:unbill-apple:end
