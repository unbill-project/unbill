import SwiftUI

// This device's id, known peers, and manual sync controls.
struct DevicesView: View {
    let console: ConsoleClient
    var onSynced: () -> Void

    @Environment(\.dismiss) private var dismiss
    @State private var loadGeneration = 0
    @State private var deviceID = ""
    @State private var peers: [SyncDevice] = []
    @State private var blePeers = BlePeerConnections()
    @State private var syncingNodeID: String?
    @State private var isSyncingAll = false
    @State private var lastSyncedAt: [String: Date] = [:]
    @State private var syncErrors: [String: String] = [:]
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

                Section {
                    if peers.isEmpty {
                        Text("No peers yet. Invite someone or join a ledger to connect devices.")
                            .foregroundStyle(.secondary)
                    }
                    ForEach(peers) { peer in
                        PeerDeviceRow(
                            peer: peer,
                            bleConnected: blePeers.connected.contains(peer.nodeID),
                            bleDiscovered: blePeers.discovered.contains(peer.nodeID),
                            syncing: syncingNodeID == peer.nodeID,
                            disabled: busy,
                            lastSyncedAt: lastSyncedAt[peer.nodeID],
                            syncError: syncErrors[peer.nodeID]
                        ) {
                            Task { await sync(peer.nodeID) }
                        }
                    }
                } header: {
                    Text("Peers")
                } footer: {
                    VStack(alignment: .leading) {
                        Text("To rename a device, open a ledger’s settings.")
                        BluetoothLegend()
                    }
                }

                if let status {
                    Section { Text(status).foregroundStyle(.secondary) }
                }
                if let error {
                    Section { Text(error).foregroundStyle(.red) }
                }
            }
            .navigationTitle("Devices")
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Done") { dismiss() }
                        .disabled(busy)
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
        .task {
            do {
                let updates = try await console.blePeerConnections()
                for await peers in updates {
                    guard !Task.isCancelled else { break }
                    blePeers = peers
                }
            } catch { /* BLE indicators are optional; IP sync remains available. */ }
            blePeers = BlePeerConnections()
        }
        .interactiveDismissDisabled(busy)
    }

    private func load() async {
        loadGeneration += 1
        let generation = loadGeneration
        do {
            let id = try await console.deviceID()
            let devices = try await console.syncDevices()
            guard generation == loadGeneration, !Task.isCancelled else { return }
            deviceID = id
            peers = devices
            error = nil
        } catch {
            guard generation == loadGeneration, !Task.isCancelled else { return }
            peers = []
            deviceID = ""
            self.error = error.localizedDescription
        }
    }

    private func sync(_ nodeID: String) async {
        guard !busy else { return }
        status = nil
        _ = await syncPeer(nodeID)
        onSynced()
    }

    private func syncPeer(_ nodeID: String) async -> Bool {
        syncingNodeID = nodeID
        syncErrors[nodeID] = nil
        defer { syncingNodeID = nil }
        do {
            try await console.syncOnce(peerNodeID: nodeID)
            lastSyncedAt[nodeID] = Date()
            return true
        } catch {
            syncErrors[nodeID] = error.localizedDescription
            return false
        }
    }

    private func syncAll() async {
        guard !busy else { return }
        let targets = peers
        isSyncingAll = true
        defer { isSyncingAll = false }
        status = nil
        var succeeded = 0
        for peer in targets {
            if await syncPeer(peer.nodeID) { succeeded += 1 }
        }
        status = "\(succeeded) synced, \(targets.count - succeeded) failed."
        onSynced()
    }
}

// Shared by the global and ledger-scoped device lists.
struct PeerDeviceRow: View {
    let peer: SyncDevice
    let bleConnected: Bool
    let bleDiscovered: Bool
    let syncing: Bool
    let disabled: Bool
    let lastSyncedAt: Date?
    let syncError: String?
    var isLocal: Bool = false
    var onRename: (() -> Void)? = nil
    var onSync: () -> Void

    var body: some View {
        HStack {
            VStack(alignment: .leading, spacing: 2) {
                HStack(spacing: 5) {
                    Text(peer.label.isEmpty ? "Unnamed device" : peer.label)
                    if isLocal {
                        Text("This device")
                            .font(.caption)
                            .foregroundStyle(.secondary)
                    }
                    if bleConnected || bleDiscovered { BluetoothBadge(connected: bleConnected) }
                }
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
                if !syncing, syncError == nil,
                   let date = lastSyncedAt {
                    Label {
                        Text("Sync complete · \(date, format: .dateTime.hour().minute())")
                    } icon: {
                        Image(systemName: "checkmark.circle")
                    }
                    .font(.caption)
                    .foregroundStyle(.secondary)
                }
                if let message = syncError {
                    Text(message).font(.caption).foregroundStyle(.red)
                }
            }
            Spacer(minLength: 12)
            HStack(spacing: 12) {
                if !isLocal {
                    Button {
                        onSync()
                    } label: {
                        ZStack {
                            Text(syncError == nil ? "Sync" : "Retry")
                                .opacity(syncing ? 0 : 1)
                            if syncing {
                                ProgressView().controlSize(.small)
                            }
                        }
                        .frame(minWidth: 48)
                    }
                    .buttonStyle(.borderedProminent)
                    .tint(Color.accentColor)
                    .disabled(disabled)
                    .accessibilityLabel(syncing ? "Syncing" : (syncError == nil ? "Sync" : "Retry"))
                }
                if let onRename {
                    Button("Rename", action: onRename)
                        .buttonStyle(.bordered)
                        .tint(.gray)
                        .disabled(disabled)
                }
            }
            .controlSize(.regular)
            .fixedSize(horizontal: true, vertical: false)
        }
    }
}

private struct BluetoothBadge: View {
    let connected: Bool
    var body: some View {
        Path { path in
            path.move(to: CGPoint(x: 2, y: 4))
            path.addLine(to: CGPoint(x: 10, y: 12))
            path.addLine(to: CGPoint(x: 6, y: 16))
            path.addLine(to: CGPoint(x: 6, y: 0))
            path.addLine(to: CGPoint(x: 10, y: 4))
            path.addLine(to: CGPoint(x: 2, y: 12))
        }
        .stroke(connected ? Color.blue : Color.secondary, style: StrokeStyle(lineWidth: 1.5, lineCap: .round, lineJoin: .round))
        .frame(width: 12, height: 16)
        .accessibilityLabel(connected ? "Connected via Bluetooth" : "Device seen, not connected")
    }
}

struct BluetoothLegend: View {
    var body: some View {
        VStack(alignment: .leading, spacing: 3) {
            HStack(spacing: 5) {
                BluetoothBadge(connected: true)
                Text("Blue: connected via Bluetooth.")
            }
            HStack(spacing: 5) {
                BluetoothBadge(connected: false)
                Text("Gray: device seen, not connected.")
            }
        }
    }
}
