import SwiftUI

struct LedgerListView: View {
    let console: ConsoleClient
    @Binding var selection: String?

    @State private var ledgers: [LedgerSummary] = []
    @State private var loadError: String?
    @State private var showingNew = false
    @State private var showingJoin = false
    @State private var showingDevices = false

    var body: some View {
        List(selection: $selection) {
            if let loadError {
                Text(loadError).foregroundStyle(.secondary)
            }
            ForEach(ledgers) { ledger in
                NavigationLink(value: ledger.id) {
                    LedgerRow(ledger: ledger)
                }
            }
        }
        .navigationTitle("Ledgers")
        .toolbar {
            ToolbarItem(placement: .primaryAction) {
                Menu {
                    Button {
                        showingNew = true
                    } label: {
                        Label("New Ledger", systemImage: "plus")
                    }
                    Button {
                        showingJoin = true
                    } label: {
                        Label("Join Ledger", systemImage: "qrcode.viewfinder")
                    }
                } label: {
                    Label("Add", systemImage: "plus")
                }
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
                ContentUnavailableView("No Ledgers", systemImage: "tray")
            }
        }
        .sheet(isPresented: $showingNew) {
            NewLedgerView(console: console) { _ in
                Task { await load() }
            }
        }
        .sheet(isPresented: $showingJoin) {
            JoinLedgerView(console: console) {
                Task { await load() }
            }
        }
        .sheet(isPresented: $showingDevices) {
            DevicesView(console: console) {
                Task { await load() }
            }
        }
        .task { await load() }
    }

    private func load() async {
        do {
            ledgers = try await console.ledgers()
            loadError = nil
        } catch {
            loadError = error.localizedDescription
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
            Text(ledger.userNames.joined(separator: ", "))
                .font(.subheadline)
                .foregroundStyle(.secondary)
                .lineLimit(1)
            Label("\(ledger.userCount)", systemImage: "person.2")
                .font(.caption)
                .foregroundStyle(.tertiary)
        }
        .padding(.vertical, 2)
    }
}
