import SwiftUI

struct LedgerDetailView: View {
    let console: ConsoleClient
    let ledgerID: String
    let ledgerName: String
    var selectedBillID: String? = nil
    var onOpenBillEditor: ((Bill?, LedgerDetail) -> Void)? = nil
    var onCloseBillEditor: (() -> Void)? = nil

    @Environment(\.scenePhase) private var scenePhase
    @State private var loadGeneration = 0
    @State private var editingBill: Bill?
    @State private var detail: LedgerDetail?
    @State private var archiveError: String?
    @State private var loadError: String?
    @State private var showingAddBill = false
    @State private var showingAddPerson = false
    @State private var showingInvite = false
    @State private var showingSettings = false
    @State private var resolvingConflict: ConflictGroup?

    init(console: ConsoleClient, ledgerID: String, ledgerName: String,
         initialDetail: LedgerDetail? = nil, selectedBillID: String? = nil,
         onOpenBillEditor: ((Bill?, LedgerDetail) -> Void)? = nil,
         onCloseBillEditor: (() -> Void)? = nil) {
        self.console = console
        self.ledgerID = ledgerID
        self.ledgerName = ledgerName
        self.selectedBillID = selectedBillID
        self.onOpenBillEditor = onOpenBillEditor
        self.onCloseBillEditor = onCloseBillEditor
        _detail = State(initialValue: initialDetail)
    }

    var body: some View {
        ZStack {
            if let detail {
                content(detail)
            } else if let loadError {
                VStack {
                    ContentUnavailableView("Couldn’t Load", systemImage: "exclamationmark.triangle", description: Text(loadError))
                    Button("Retry") { Task { await load() } }
                }
            } else {
                ProgressView()
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .navigationTitle(detail?.summary.name ?? ledgerName)
        .toolbar {
            ToolbarItem(placement: .primaryAction) {
                Menu {
                    Button {
                        showingAddPerson = true
                    } label: {
                        Label("Add Person", systemImage: "person.badge.plus")
                    }
                    Button {
                        if let detail, let onOpenBillEditor { onOpenBillEditor(nil, detail) }
                        else { showingAddBill = true }
                    } label: {
                        Label("Add Bill", systemImage: "dollarsign.circle")
                    }
                    // A bill needs users to pay/split it.
                    .keyboardShortcut("n", modifiers: [.command, .shift])
                    .disabled(detail?.users.isEmpty ?? true)

                } label: {
                    Label("Add", systemImage: "plus")
                }
                .disabled(detail == nil)
            }
            ToolbarItemGroup(placement: .primaryAction) {
                Button { showingInvite = true } label: {
                    Label("Invite via QR", systemImage: "qrcode")
                }
                .help("Invite via QR")
                .disabled(detail == nil)
                Button { showingSettings = true } label: {
                    Label("Ledger Settings", systemImage: "gearshape")
                }
                .help("Ledger Settings")
                .disabled(detail == nil)
            }
        }
        .sheet(isPresented: $showingAddBill) {
            if let detail {
                AddBillView(
                    console: console,
                    ledgerID: ledgerID,
                    currency: detail.summary.currency,
                    users: detail.users
                ) {
                    Task { await load() }
                }
            }
        }
        .sheet(item: $editingBill) { bill in
            if let detail {
                AddBillView(console: console, ledgerID: ledgerID, currency: detail.summary.currency,
                    users: detail.users, bill: bill) { Task { await load() } }
            }
        }
        .sheet(isPresented: $showingAddPerson) {
            AddUserView(
                console: console,
                ledgerID: ledgerID,
                existingUserIDs: Set(detail?.users.map(\.userID) ?? [])
            ) {
                Task { await load() }
            }
        }
        .sheet(isPresented: $showingInvite) {
            InviteView(
                console: console,
                ledgerID: ledgerID,
                ledgerName: detail?.summary.name ?? "Ledger"
            )
        }
        .sheet(item: $resolvingConflict) { group in
            ConflictResolutionView(
                console: console,
                ledgerID: ledgerID,
                currency: detail?.summary.currency ?? "USD",
                group: group
            ) {
                Task { await load() }
            }
        }
        .sheet(isPresented: $showingSettings) {
            if let detail {
                LedgerSettingsView(console: console, detail: detail) {
                    Task { await load() }
                }
            }
        }
        .refreshable { await load() }
        .onChange(of: scenePhase) { _, phase in
            if phase == .active { Task { await load() } }
        }
        .task {
            let updates = await console.ledgerUpdates()
            if detail == nil { await load() }
            for await updatedID in updates {
                if Task.isCancelled { break }
                if updatedID == nil || updatedID == ledgerID {
                    await load()
                }
            }
        }
    }

    private func load() async {
        loadGeneration += 1
        let generation = loadGeneration
        do {
            let loaded = try await console.ledgerDetail(id: ledgerID)
            guard generation == loadGeneration, !Task.isCancelled else { return }
            detail = loaded
            loadError = nil
        } catch {
            guard generation == loadGeneration, !Task.isCancelled else { return }
            detail = nil
            loadError = error.localizedDescription
        }
    }

    private func personRow(_ user: User) -> some View {
        HStack {
            Label(user.displayName, systemImage: "person")
            Spacer()
            Button(user.archived ? "Restore" : "Archive") {
                Task {
                    do {
                        try await console.setUserArchived(ledgerID: ledgerID, userID: user.userID, archived: !user.archived)
                        archiveError = nil
                        await load()
                    } catch { archiveError = error.localizedDescription }
                }
            }.buttonStyle(.borderless)
        }
    }

    private func openBill(_ bill: Bill, detail: LedgerDetail) {
        if let onOpenBillEditor { onOpenBillEditor(bill, detail) }
        else { editingBill = bill }
    }

    private func content(_ detail: LedgerDetail) -> some View {
        let currency = detail.summary.currency
        let selection = Binding<String?>(get: { selectedBillID }, set: { id in
            guard id != selectedBillID else { return }
            if let id, let bill = detail.bills.first(where: { $0.id == id }) {
                openBill(bill, detail: detail)
            } else if id == nil {
                onCloseBillEditor?()
            }
        })
        return List(selection: onOpenBillEditor == nil ? nil : selection) {
            Section("People") {
                if detail.users.isEmpty {
                    Text("No people yet").foregroundStyle(.secondary)
                }
                ForEach(detail.users.filter { !$0.archived }) { user in
                    personRow(user)
                }
                if detail.users.contains(where: { $0.archived }) {
                    DisclosureGroup("Archived") {
                        ForEach(detail.users.filter { $0.archived }) { user in personRow(user) }
                    }
                }
                if let archiveError { Text(archiveError).foregroundStyle(.red) }
            }.selectionDisabled()

            if !detail.conflicts.isEmpty {
                Section("Conflicts") {
                    ForEach(detail.conflicts) { group in
                        Button {
                            resolvingConflict = group
                        } label: {
                            HStack {
                                Label {
                                    Text(group.conflicting.first?.description ?? "Conflicting bill")
                                        .foregroundStyle(.primary)
                                } icon: {
                                    Image(systemName: "exclamationmark.triangle.fill")
                                        .foregroundStyle(.orange)
                                }
                                Spacer()
                                Text("\(group.conflicting.count) versions")
                                    .font(.caption)
                                    .foregroundStyle(.secondary)
                                Image(systemName: "chevron.right")
                                    .font(.caption)
                                    .foregroundStyle(.tertiary)
                            }
                        }
                    }
                }.selectionDisabled()
            }

            Section("Settlement") {
                if detail.settlement.isEmpty {
                    Text("All settled up").foregroundStyle(.secondary)
                }
                ForEach(detail.settlement, id: \.self) { txn in
                    HStack {
                        Text(txn.fromName)
                        Image(systemName: "arrow.right")
                            .font(.caption)
                            .foregroundStyle(.secondary)
                        Text(txn.toName)
                        Spacer()
                        Text(Money.string(cents: txn.amountCents, currency: currency))
                            .monospacedDigit()
                    }
                }
            }.selectionDisabled()

            Section("Bills") {
                if detail.bills.isEmpty {
                    Text("No bills yet").foregroundStyle(.secondary)
                }
                ForEach(detail.bills) { bill in
                    HStack {
                        VStack(alignment: .leading, spacing: 4) {
                            HStack {
                                Text(bill.description)
                                Spacer()
                                Text(Money.string(cents: bill.amountCents, currency: currency)).monospacedDigit()
                            }.foregroundStyle(.primary)
                            Text("Paid: " + bill.payers.map { "\($0.displayName) (\($0.shares))" }.joined(separator: ", "))
                            Text("Split: " + bill.payees.map { "\($0.displayName) (\($0.shares))" }.joined(separator: ", "))
                            Text(Date(timeIntervalSince1970: Double(bill.createdAtMs) / 1000), format: .dateTime.year().month().day().hour().minute())
                        }.font(.subheadline).foregroundStyle(.secondary)
                        #if targetEnvironment(macCatalyst)
                        Image(systemName: "chevron.right")
                            .font(.title3.weight(.semibold))
                            .foregroundStyle(.secondary)
                            .padding(.leading, 8)
                        #endif
                    }
                    .tag(bill.id)
                    .contentShape(Rectangle())
                    #if targetEnvironment(macCatalyst)
                    .accessibilityAction { openBill(bill, detail: detail) }
                    #endif
                    #if !targetEnvironment(macCatalyst)
                    .swipeActions(edge: .trailing, allowsFullSwipe: false) {
                        Button { openBill(bill, detail: detail) } label: {
                            Label("Edit", systemImage: "pencil")
                        }.tint(.gray)
                    }
                    #endif
                    .contextMenu {
                        Button { openBill(bill, detail: detail) } label: {
                            Label("Edit", systemImage: "pencil")
                        }
                    }
                }
            }
        }
    }
}
