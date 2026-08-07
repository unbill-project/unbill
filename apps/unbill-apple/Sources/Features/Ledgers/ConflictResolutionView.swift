import SwiftUI

// Resolve one conflict group: the competing bill versions are shown; pick the
// one to keep and commit. The commit supersedes every competing version with a
// merge amendment (handled by the Rust core).
struct ConflictResolutionView: View {
    let console: ConsoleClient
    let ledgerID: String
    let currency: String
    let group: ConflictGroup
    var onResolved: () -> Void

    @Environment(\.dismiss) private var dismiss
    @State private var selectedBillID: String
    @State private var isSaving = false
    @State private var error: String?

    init(
        console: ConsoleClient, ledgerID: String, currency: String,
        group: ConflictGroup, onResolved: @escaping () -> Void
    ) {
        self.console = console
        self.ledgerID = ledgerID
        self.currency = currency
        self.group = group
        self.onResolved = onResolved
        _selectedBillID = State(initialValue: group.conflicting.first?.id ?? "")
    }

    var body: some View {
        NavigationStack {
            List {
                Section {
                    Text("These versions of the same bill conflict. Choose the one to keep — the others will be superseded.")
                        .font(.subheadline)
                        .foregroundStyle(.secondary)
                }

                Section("Competing Versions") {
                    ForEach(group.conflicting) { bill in
                        Button {
                            selectedBillID = bill.id
                        } label: {
                            HStack(alignment: .top) {
                                Image(systemName: selectedBillID == bill.id ? "largecircle.fill.circle" : "circle")
                                    .foregroundStyle(selectedBillID == bill.id ? AnyShapeStyle(.tint) : AnyShapeStyle(.secondary))
                                VStack(alignment: .leading, spacing: 2) {
                                    HStack {
                                        Text(bill.description).foregroundStyle(.primary)
                                        Spacer()
                                        Text(Money.string(cents: bill.amountCents, currency: currency))
                                            .monospacedDigit()
                                            .foregroundStyle(.primary)
                                    }
                                    Text(bill.payers.map(\.displayName).joined(separator: ", ") + " paid · split "
                                        + bill.payees.map(\.displayName).joined(separator: ", "))
                                        .font(.caption)
                                        .foregroundStyle(.secondary)
                                }
                            }
                        }
                    }
                }

                if let error {
                    Section { Text(error).foregroundStyle(.red) }
                }
            }
            .navigationTitle("Resolve Conflict")
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Cancel") { dismiss() }
                }
                ToolbarItem(placement: .confirmationAction) {
                    Button("Keep") { Task { await resolve() } }
                        .disabled(selectedBillID.isEmpty || isSaving)
                }
            }
        }
    }

    private func resolve() async {
        isSaving = true
        error = nil
        do {
            try await console.resolveConflict(
                ledgerID: ledgerID,
                selectedBillID: selectedBillID,
                conflictingBillIDs: group.conflicting.map(\.id)
            )
            onResolved()
            dismiss()
        } catch {
            self.error = error.localizedDescription
        }
        isSaving = false
    }
}
