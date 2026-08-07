import SwiftUI

struct LedgerDetailView: View {
    let console: ConsoleClient
    let ledgerID: String

    @State private var detail: LedgerDetail?
    @State private var loadError: String?
    @State private var showingAddBill = false
    @State private var showingAddPerson = false
    @State private var showingInvite = false
    @State private var resolvingConflict: ConflictGroup?

    var body: some View {
        Group {
            if let detail {
                content(detail)
            } else if let loadError {
                ContentUnavailableView("Couldn’t Load", systemImage: "exclamationmark.triangle", description: Text(loadError))
            } else {
                ProgressView()
            }
        }
        .navigationTitle(detail?.summary.name ?? "Ledger")
        .toolbar {
            ToolbarItem(placement: .primaryAction) {
                Menu {
                    Button {
                        showingAddPerson = true
                    } label: {
                        Label("Add Person", systemImage: "person.badge.plus")
                    }
                    Button {
                        showingAddBill = true
                    } label: {
                        Label("Add Bill", systemImage: "dollarsign.circle")
                    }
                    // A bill needs users to pay/split it.
                    .disabled(detail?.users.isEmpty ?? true)

                    Divider()

                    Button {
                        showingInvite = true
                    } label: {
                        Label("Invite via QR", systemImage: "qrcode")
                    }
                } label: {
                    Label("Add", systemImage: "plus")
                }
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
        .task { await load() }
    }

    private func load() async {
        do {
            detail = try await console.ledgerDetail(id: ledgerID)
            loadError = nil
        } catch {
            loadError = error.localizedDescription
        }
    }

    private func content(_ detail: LedgerDetail) -> some View {
        let currency = detail.summary.currency
        return List {
            Section("People") {
                if detail.users.isEmpty {
                    Text("No people yet").foregroundStyle(.secondary)
                }
                ForEach(detail.users) { user in
                    Label(user.displayName, systemImage: "person")
                }
            }

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
                }
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
            }

            Section("Bills") {
                if detail.bills.isEmpty {
                    Text("No bills yet").foregroundStyle(.secondary)
                }
                ForEach(detail.bills) { bill in
                    HStack {
                        VStack(alignment: .leading) {
                            Text(bill.description)
                            Text(bill.payers.map(\.displayName).joined(separator: ", ") + " paid")
                                .font(.caption)
                                .foregroundStyle(.secondary)
                        }
                        Spacer()
                        Text(Money.string(cents: bill.amountCents, currency: currency))
                            .monospacedDigit()
                    }
                }
            }
        }
    }
}
