import SwiftUI

struct LedgerListView: View {
    let console: ConsoleClient
    @Binding var selection: String?
    var onJoin: () -> Void

    @Environment(\.scenePhase) private var scenePhase
    @State private var loadGeneration = 0
    @State private var loading = true
    @State private var ledgers: [LedgerSummary] = []
    @State private var loadError: String?
    @State private var showingNew = false
    @State private var showingDevices = false

    var body: some View {
        List(selection: $selection) {
            if let loadError {
                Text(loadError).foregroundStyle(.red)
                Button("Retry") { Task { await load() } }
            }
            ForEach(ledgers) { ledger in
                NavigationLink(value: ledger.id) {
                    LedgerRow(ledger: ledger)
                }
            }
        }
        .navigationTitle("Ledgers")
        .toolbar {
            ToolbarItemGroup(placement: .primaryAction) {
                Button { showingNew = true } label: {
                    Label("New Ledger", systemImage: "plus")
                }
                .keyboardShortcut("n", modifiers: .command)
                .help("New Ledger")
                Button { onJoin() } label: {
                    Label("Join Ledger", systemImage: "qrcode.viewfinder")
                }
                .help("Join Ledger")
            }
            ToolbarItem(placement: .cancellationAction) {
                Button {
                    showingDevices = true
                } label: {
                    Label("Devices", systemImage: "laptopcomputer.and.iphone")
                }
            }
        }
        .overlay {
            if ledgers.isEmpty && loadError == nil {
                if loading { ProgressView() }
                else { ContentUnavailableView("No Ledgers", systemImage: "tray") }
            }
        }
        .sheet(isPresented: $showingNew) {
            NewLedgerView(console: console) { _ in
                Task { await load() }
            }
        }
        .sheet(isPresented: $showingDevices) {
            DevicesView(console: console) {
                Task { await load() }
            }
        }
        .refreshable { await load() }
        .onChange(of: scenePhase) { _, phase in
            if phase == .active { Task { await load() } }
        }
        .task {
            let updates = await console.ledgerUpdates()
            await load()
            for await _ in updates {
                if Task.isCancelled { break }
                await load()
            }
        }
    }

    private func load() async {
        loadGeneration += 1
        let generation = loadGeneration
        loading = true
        do {
            let loaded = try await console.ledgers()
            guard generation == loadGeneration, !Task.isCancelled else { return }
            ledgers = loaded
            loadError = nil
            loading = false
        } catch {
            guard generation == loadGeneration, !Task.isCancelled else { return }
            ledgers = []
            loadError = error.localizedDescription
            loading = false
        }
    }
}

private struct LedgerRow: View {
    let ledger: LedgerSummary

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            HStack {
                Text(ledger.name).font(.headline)
                Spacer()
                Text(ledger.currency)
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
            Text(ledger.userNames.prefix(3).joined(separator: ", ") + (ledger.userCount > 3 ? " +\(ledger.userCount - 3)" : ""))
                .font(.subheadline)
                .foregroundStyle(.secondary)
                .lineLimit(1)
            if let timestamp = ledger.latestBillAtMs {
                Text(Date(timeIntervalSince1970: Double(timestamp) / 1000), format: .dateTime.year().month().day().hour().minute())
                    .font(.caption).foregroundStyle(.secondary)
            }
            Label("\(ledger.userCount)", systemImage: "person.2")
                .font(.caption)
                .foregroundStyle(.tertiary)
        }
        .padding(.vertical, 2)
    }
}
