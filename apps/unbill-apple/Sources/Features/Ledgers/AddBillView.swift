import SwiftUI

// Sheet for adding a bill to a ledger: description, amount, who paid, and who
// it's split between (equal shares). Calls console.saveBill (real Rust or mock).
struct AddBillView: View {
    let console: ConsoleClient
    let ledgerID: String
    let currency: String
    let users: [User]
    var onSaved: () -> Void

    @Environment(\.dismiss) private var dismiss
    @State private var description = ""
    @State private var amount = ""
    @State private var payerID: String = ""
    @State private var splitIDs: Set<String> = []
    @State private var isSaving = false
    @State private var error: String?

    private var trimmedDescription: String {
        description.trimmingCharacters(in: .whitespacesAndNewlines)
    }

    // Parse the major-unit amount into integer minor units (cents).
    private var amountCents: Int64? {
        guard let value = Decimal(string: amount.replacingOccurrences(of: ",", with: ".")),
              value > 0 else { return nil }
        return NSDecimalNumber(decimal: value * 100).int64Value
    }

    private var canSave: Bool {
        !trimmedDescription.isEmpty && amountCents != nil && !payerID.isEmpty
            && !splitIDs.isEmpty && !isSaving
    }

    var body: some View {
        NavigationStack {
            Form {
                Section {
                    TextField("Description", text: $description)
                    HStack {
                        Text(currency).foregroundStyle(.secondary)
                        TextField("Amount", text: $amount)
                            .keyboardType(.decimalPad)
                            .multilineTextAlignment(.trailing)
                    }
                }

                Section("Paid by") {
                    ForEach(users.filter { !$0.archived }) { user in payerRow(user) }
                    if users.contains(where: { $0.archived }) {
                        DisclosureGroup("Archived") {
                            ForEach(users.filter { $0.archived }) { user in payerRow(user) }
                        }
                    }
                }

                Section("Split between") {
                    ForEach(users.filter { !$0.archived }) { user in payeeRow(user) }
                    if users.contains(where: { $0.archived }) {
                        DisclosureGroup("Archived") {
                            ForEach(users.filter { $0.archived }) { user in payeeRow(user) }
                        }
                    }
                }

                if let error {
                    Section { Text(error).foregroundStyle(.red) }
                }
            }
            .navigationTitle("Add Bill")
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Cancel") { dismiss() }
                }
                ToolbarItem(placement: .confirmationAction) {
                    Button("Save") { Task { await save() } }.disabled(!canSave)
                }
            }
            .onAppear {
                // Default to active users; archived users require explicit selection.
                if payerID.isEmpty { payerID = users.first(where: { !$0.archived })?.userID ?? "" }
                if splitIDs.isEmpty { splitIDs = Set(users.filter { !$0.archived }.map(\.userID)) }
            }
        }
    }

    private func payerRow(_ user: User) -> some View {
        Button { payerID = user.userID } label: {
            HStack {
                Text(user.displayName).foregroundStyle(.primary)
                Spacer()
                if payerID == user.userID { Image(systemName: "checkmark").foregroundStyle(.tint) }
            }
        }
    }

    private func payeeRow(_ user: User) -> some View {
        Button { toggle(user.userID) } label: {
            HStack {
                Text(user.displayName).foregroundStyle(.primary)
                Spacer()
                if splitIDs.contains(user.userID) { Image(systemName: "checkmark").foregroundStyle(.tint) }
            }
        }
    }

    private func toggle(_ id: String) {
        if splitIDs.contains(id) { splitIDs.remove(id) } else { splitIDs.insert(id) }
    }

    private func save() async {
        guard let cents = amountCents else { return }
        isSaving = true
        error = nil
        do {
            try await console.saveBill(
                ledgerID: ledgerID,
                description: trimmedDescription,
                amountCents: cents,
                payerUserIDs: [payerID],
                payeeUserIDs: users.map(\.userID).filter { splitIDs.contains($0) }
            )
            onSaved()
            dismiss()
        } catch {
            self.error = error.localizedDescription
        }
        isSaving = false
    }
}
