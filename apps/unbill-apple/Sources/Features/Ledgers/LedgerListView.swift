import SwiftUI

struct LedgerListView: View {
    let console: ConsoleClient
    @Binding var selection: String?
    var onJoin: () -> Void
    var onOpenLedger: (LedgerDetail) -> Void
    @State private var openingLedgerID: String?
    @State private var openingTask: Task<Void, Never>?
    @State private var openingError: String?

    @Environment(\.scenePhase) private var scenePhase
    @State private var loadGeneration = 0
    @State private var loading = true
    @State private var ledgers: [LedgerSummary] = []
    @State private var loadError: String?
    @State private var archivedExpanded = false
    @State private var showingNew = false
    @State private var showingDevices = false

    var body: some View {
        List(selection: Binding(get: { selection }, set: { _ in })) {
            if let loadError {
                Text(loadError).foregroundStyle(.red)
                Button("Retry") { Task { await load() } }
            }
            ForEach(ledgers.filter { !$0.archived }) { ledger in
                ledgerButton(ledger)
            }
            let archived = ledgers.filter { $0.archived }
            if !archived.isEmpty {
                DisclosureGroup("Archived (\(archived.count))", isExpanded: $archivedExpanded) {
                    ForEach(archived) { ledger in
                        ledgerButton(ledger)
                    }
                }
            }
        }
        .navigationTitle("Ledgers")
        .alert("Couldn’t Open Ledger", isPresented: Binding(
            get: { openingError != nil }, set: { if !$0 { openingError = nil } }
        )) {
            Button("OK", role: .cancel) { openingError = nil }
        } message: {
            Text(openingError ?? "")
        }
        .onDisappear {
            openingTask?.cancel()
        }
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
        .sheet(isPresented: $showingDevices, onDismiss: { Task { await load() } }) {
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

    private func ledgerButton(_ ledger: LedgerSummary) -> some View {
        Button {
            guard openingLedgerID == nil else { return }
            openingLedgerID = ledger.id
            openingTask = Task {
                defer {
                    openingLedgerID = nil
                    openingTask = nil
                }
                do {
                    let detail = try await console.ledgerDetail(id: ledger.id)
                    guard !Task.isCancelled else { return }
                    onOpenLedger(detail)
                } catch {
                    guard !Task.isCancelled else { return }
                    openingError = error.localizedDescription
                }
            }
        } label: {
            HStack {
                LedgerRow(ledger: ledger)
                if openingLedgerID == ledger.id {
                    ProgressView().controlSize(.small)
                        .accessibilityLabel("Loading ledger")
                } else {
                    Image(systemName: "chevron.right")
                        .font(.body.weight(.semibold))
                        .foregroundStyle(.tertiary)
                }
            }
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .tag(ledger.id)
    }

    private func load() async {
        loadGeneration += 1
        let generation = loadGeneration
        loading = true
        do {
            let loaded = try await console.ledgers()
            guard generation == loadGeneration, !Task.isCancelled else { return }
            if let selection {
                let previous = ledgers.first { $0.id == selection }
                let current = loaded.first { $0.id == selection }
                if current == nil || (previous?.archived == false && current?.archived == true) {
                    self.selection = nil
                }
            }
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
