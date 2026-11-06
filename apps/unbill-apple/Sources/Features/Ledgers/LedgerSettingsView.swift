import SwiftUI
import UIKit

// sirno:witness:unbill-apple:begin
struct LedgerSettingsView: View {
    let console: ConsoleClient
    let detail: LedgerDetail
    var onRenamed: () -> Void
    @Environment(\.dismiss) private var dismiss
    @State private var devices: [SyncDevice]
    @State private var localNodeID = ""
    @State private var renameTarget: SyncDevice?
    @State private var renamingLedger = false
    @State private var copied = false
    @State private var blePeers = BlePeerConnections()
    @State private var syncingNodeID: String?
    @State private var lastSyncedAt: [String: Date] = [:]
    @State private var syncErrors: [String: String] = [:]

    @State private var archiving = false
    @State private var archiveError: String?
    @State private var cloning = false
    @State private var cloneError: String?
    @State private var cloned = false

    private var busy: Bool { syncingNodeID != nil || archiving || cloning }

    init(console: ConsoleClient, detail: LedgerDetail, onRenamed: @escaping () -> Void) {
        self.console = console
        self.detail = detail
        self.onRenamed = onRenamed
        _devices = State(initialValue: detail.devices)
    }

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
                Section {
                    if devices.isEmpty { Text("No authorized devices").foregroundStyle(.secondary) }
                    ForEach(devices) { peer in
                        PeerDeviceRow(
                            peer: peer,
                            bleConnected: blePeers.connected.contains(peer.nodeID),
                            bleDiscovered: blePeers.discovered.contains(peer.nodeID),
                            syncing: syncingNodeID == peer.nodeID,
                            disabled: busy,
                            lastSyncedAt: lastSyncedAt[peer.nodeID],
                            syncError: syncErrors[peer.nodeID],
                            isLocal: peer.nodeID == localNodeID,
                            onRename: { renameTarget = peer }
                        ) {
                            Task { await sync(peer.nodeID) }
                        }
                    }
                } header: {
                    Text("Ledger Devices")
                } footer: {
                    VStack(alignment: .leading) {
                        Text("Names are shared with other devices in this ledger.")
                        BluetoothLegend()
                    }
                }
                Section {
                    // sirno:witness:ledger-rename:begin
                    Button("Rename Ledger") { renamingLedger = true }
                        .disabled(busy)
                    // sirno:witness:ledger-rename:end
                    // sirno:witness:ledger-clone:begin
                    Button(cloning ? "Cloning Ledger…" : "Clone Ledger") {
                        Task {
                            guard !busy else { return }
                            cloning = true
                            cloned = false
                            cloneError = nil
                            defer { cloning = false }
                            do {
                                try await console.cloneLedger(ledgerID: detail.summary.ledgerID)
                                cloned = true
                            } catch { cloneError = error.localizedDescription }
                        }
                    }
                    .disabled(busy)
                    // sirno:witness:ledger-clone:end
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
                    if cloned { Text("Ledger cloned.").foregroundStyle(.secondary) }
                    if let cloneError { Text(cloneError).foregroundStyle(.red) }
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
        .task {
            localNodeID = (try? await console.deviceID()) ?? ""
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
        .sheet(isPresented: $renamingLedger) {
            RenameNameSheet(name: detail.summary.name, target: .ledger) { name in
                try await console.renameLedger(ledgerID: detail.summary.ledgerID, name: name)
                onRenamed()
            }
        }
        .sheet(item: $renameTarget) { peer in
            RenameNameSheet(name: peer.label, target: .device(ledgerName: detail.summary.name)) { name in
                try await console.renameDevice(ledgerID: detail.summary.ledgerID, nodeID: peer.nodeID, name: name)
                devices = devices.map { device in
                    guard device.nodeID == peer.nodeID else { return device }
                    return SyncDevice(nodeID: device.nodeID, label: name.trimmingCharacters(in: .whitespacesAndNewlines), ledgerNames: device.ledgerNames)
                }
                onRenamed()
            }
        }
        .onChange(of: detail.devices) { _, updated in devices = updated }
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

private struct RenameNameSheet: View {
    enum Target {
        case ledger
        case device(ledgerName: String)
    }

    let name: String
    let target: Target
    let onSave: (String) async throws -> Void

    @Environment(\.dismiss) private var dismiss
    @State private var editedName: String
    @State private var saving = false
    @State private var error: String?

    init(name: String, target: Target, onSave: @escaping (String) async throws -> Void) {
        self.name = name
        self.target = target
        self.onSave = onSave
        _editedName = State(initialValue: name)
    }

    private var trimmedName: String { editedName.trimmingCharacters(in: .whitespacesAndNewlines) }

    private var title: String {
        switch target {
        case .ledger: "Rename Ledger"
        case .device: "Rename Device"
        }
    }

    private var fieldLabel: String {
        switch target {
        case .ledger: "Ledger name"
        case .device: "Device name"
        }
    }

    private var footer: String {
        switch target {
        case .ledger: "The name is shared with other devices when the ledger syncs."
        case .device(let ledgerName): "Other devices in \(ledgerName) will see this name after syncing."
        }
    }

    private var lengthError: String? {
        switch target {
        case .ledger:
            InputValidation.ledgerName(trimmedName)
        case .device:
            InputValidation.deviceName(trimmedName)
        }
    }

    var body: some View {
        NavigationStack {
            Form {
                Section {
                    TextField(fieldLabel, text: $editedName)
                        .textInputAutocapitalization(.words)
                } footer: {
                    Text(footer)
                }
                if let lengthError { Text(lengthError).foregroundStyle(.red) }
                if let error { Text(error).foregroundStyle(.red) }
            }
            .navigationTitle(title)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Cancel") { dismiss() }.disabled(saving)
                }
                ToolbarItem(placement: .confirmationAction) {
                    Button("Save") {
                        Task {
                            saving = true
                            defer { saving = false }
                            do {
                                try await onSave(trimmedName)
                                dismiss()
                            } catch { self.error = error.localizedDescription }
                        }
                    }
                    .disabled(saving || lengthError != nil || trimmedName == name)
                }
            }
        }
        .interactiveDismissDisabled(saving)
    }
}
// sirno:witness:unbill-apple:end
