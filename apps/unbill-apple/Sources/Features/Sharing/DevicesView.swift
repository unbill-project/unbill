import SwiftUI

// This device's id, known peers, and manual sync controls.
struct DevicesView: View {
    let console: ConsoleClient
    var onSynced: () -> Void

    @Environment(\.dismiss) private var dismiss
    @State private var deviceID = ""
    @State private var peers: [SyncDevice] = []
    @State private var syncingNodeID: String?
    @State private var isSyncingAll = false
    @State private var status: String?
    @State private var error: String?

    private var busy: Bool { isSyncingAll || syncingNodeID != nil }

    var body: some View {
        NavigationStack {
            List {
                Section("This Device") {
                    LabeledContent("Node ID") {
                        Text(deviceID)
                            .font(.footnote.monospaced())
                            .lineLimit(1)
                            .truncationMode(.middle)
                            .textSelection(.enabled)
                    }
                }

                Section("Peers") {
                    if peers.isEmpty {
                        Text("No peers yet. Invite someone or join a ledger to connect devices.")
                            .foregroundStyle(.secondary)
                    }
                    ForEach(peers) { peer in
                        HStack {
                            VStack(alignment: .leading, spacing: 2) {
                                Text(peer.label.isEmpty ? "Unnamed device" : peer.label)
                                Text(peer.nodeID)
                                    .font(.caption2.monospaced())
                                    .foregroundStyle(.secondary)
                                    .lineLimit(1)
                                    .truncationMode(.middle)
                                    .textSelection(.enabled)
                                if !peer.ledgerNames.isEmpty {
                                    Text(peer.ledgerNames.joined(separator: ", "))
                                        .font(.caption)
                                        .foregroundStyle(.tertiary)
                                }
                            }
                            Spacer()
                            if syncingNodeID == peer.nodeID {
                                ProgressView()
                            } else {
                                Button("Sync") { Task { await sync(peer.nodeID) } }
                                    .buttonStyle(.bordered)
                                    .disabled(busy)
                            }
                        }
                    }
                }

                if let status {
                    Section { Label(status, systemImage: "checkmark.circle").foregroundStyle(.secondary) }
                }
                if let error {
                    Section { Text(error).foregroundStyle(.red) }
                }
            }
            .navigationTitle("Devices")
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Done") { dismiss() }
                }
                ToolbarItem(placement: .primaryAction) {
                    Button { Task { await syncAll() } } label: {
                        Label("Sync All", systemImage: "arrow.triangle.2.circlepath")
                    }
                    .disabled(peers.isEmpty || busy)
                }
            }
            .task { await load() }
            .refreshable { await load() }
        }
    }

    private func load() async {
        do {
            deviceID = try await console.deviceID()
            peers = try await console.syncDevices()
        } catch {
            self.error = error.localizedDescription
        }
    }

    private func sync(_ nodeID: String) async {
        syncingNodeID = nodeID
        error = nil
        status = nil
        do {
            try await console.syncOnce(peerNodeID: nodeID)
            status = "Synced."
            onSynced()
        } catch {
            self.error = error.localizedDescription
        }
        syncingNodeID = nil
    }

    private func syncAll() async {
        isSyncingAll = true
        error = nil
        status = nil
        for peer in peers {
            do {
                try await console.syncOnce(peerNodeID: peer.nodeID)
            } catch {
                self.error = error.localizedDescription
            }
        }
        if error == nil { status = "Synced \(peers.count) peer(s)." }
        onSynced()
        isSyncingAll = false
    }
}
