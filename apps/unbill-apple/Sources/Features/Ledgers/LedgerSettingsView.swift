import SwiftUI
import UIKit

// sirno:witness:unbill-apple:begin
struct LedgerSettingsView: View {
    let console: ConsoleClient
    let detail: LedgerDetail
    @Environment(\.dismiss) private var dismiss
    @State private var copied = false
    @State private var syncing = false
    @State private var syncError: String?

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
                        HStack {
                            VStack(alignment: .leading) {
                                Text(peer.label.isEmpty ? "Unnamed device" : peer.label)
                                Text(peer.nodeID).font(.caption.monospaced()).textSelection(.enabled)
                            }
                            Spacer()
                            Button("Sync") {
                                Task {
                                    syncing = true
                                    syncError = nil
                                    do { try await console.syncOnce(peerNodeID: peer.nodeID) }
                                    catch { syncError = error.localizedDescription }
                                    syncing = false
                                }
                            }.disabled(syncing)
                        }
                    }
                    if let syncError { Text(syncError).foregroundStyle(.red) }
                }
            }
            .navigationTitle("Ledger Settings")
            .toolbar {
                ToolbarItem(placement: .confirmationAction) {
                    Button("Done") { dismiss() }
                }
            }
        }
        .onChange(of: detail.emojiFingerprint) { _, _ in copied = false }
    }
}
// sirno:witness:unbill-apple:end
