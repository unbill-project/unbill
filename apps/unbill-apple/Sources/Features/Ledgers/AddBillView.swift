import SwiftUI

// Shared editor for new bills and amendments; Rust owns split calculations.
struct AddBillView: View {
    let console: ConsoleClient
    let ledgerID: String
    let currency: String
    let users: [User]
    var bill: Bill? = nil
    var embedded = false
    var onCancel: (() -> Void)? = nil
    var onSaved: () -> Void

    @Environment(\.dismiss) private var dismiss
    @State private var description = ""
    @State private var amount = ""
    @State private var payerWeights: [String: String] = [:]
    @State private var payeeWeights: [String: String] = [:]
    @State private var customPayers = false
    @State private var customPayees = false
    @State private var initialized = false
    @State private var isSaving = false
    @State private var error: String?
    @State private var previewError: String?
    @State private var preview: BillSplitPreview?
    @State private var previewID = newBillId()

    private var draftKey: String {
        ([amount, String(customPayers), String(customPayees)] + users.flatMap {
            [$0.userID, payerWeights[$0.userID] ?? "", payeeWeights[$0.userID] ?? ""]
        }).joined(separator: "|")
    }

    private func shares(_ weights: [String: String], custom: Bool) throws -> [BillShareInput] {
        let inputs = try users.filter { weights[$0.userID] != nil }.map { user -> BillShareInput in
            guard let weight = custom ? UInt32(weights[user.userID] ?? "") : 1, weight > 0 else {
                throw BillInputError.invalid("Shares must be whole numbers from 1 to 4294967295.")
            }
            return BillShareInput(userID: user.userID, shares: custom ? weight : 1)
        }
        guard !inputs.isEmpty else { throw BillInputError.invalid("Select at least one payer and one participant.") }
        return inputs
    }

    @ViewBuilder var body: some View {
        if embedded { editor }
        else { NavigationStack { editor } }
    }

    private var editor: some View {
        Group {
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
                    Picker("Payment shares", selection: $customPayers) {
                        Text("Equal shares").tag(false)
                        Text("Custom shares").tag(true)
                    }.pickerStyle(.segmented)
                    participantRows(payers: true)
                }
                Section("Split between") {
                    Picker("Participant shares", selection: $customPayees) {
                        Text("Equal shares").tag(false)
                        Text("Custom shares").tag(true)
                    }.pickerStyle(.segmented)
                    participantRows(payers: false)
                }
                if let previewError {
                    Section { Text(previewError).foregroundStyle(.red) }
                }
                if let bill {
                    Section("Original bill") {
                        Text(Date(timeIntervalSince1970: Double(bill.createdAtMs) / 1000), format: .dateTime.year().month().day().hour().minute())
                    }
                }
                if let error { Section { Text(error).foregroundStyle(.red) } }
            }
            .disabled(isSaving)
            .navigationTitle(bill == nil ? "Add Bill" : "Edit Bill")
            .toolbar {
                ToolbarItem(placement: .cancellationAction) { Button("Cancel") { if let onCancel { onCancel() } else { dismiss() } }.keyboardShortcut(.cancelAction) }
                ToolbarItem(placement: .confirmationAction) {
                    Button("Save") { Task { await save() } }.keyboardShortcut("s", modifiers: .command)
                        .disabled(description.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty || preview == nil || isSaving)
                }
            }
            .onAppear { seed() }
            .task(id: draftKey) { await updatePreview() }
        }
    }

    @ViewBuilder private func participantRows(payers: Bool) -> some View {
        ForEach(users.filter { !$0.archived }) { participant($0, payers: payers) }
        if users.contains(where: { $0.archived }) {
            DisclosureGroup("Archived") {
                ForEach(users.filter { $0.archived }) { participant($0, payers: payers) }
            }
        }
    }

    private func participant(_ user: User, payers: Bool) -> some View {
        let selected = (payers ? payerWeights : payeeWeights)[user.userID] != nil
        let custom = payers ? customPayers : customPayees
        return VStack(alignment: .leading) {
            Toggle(user.displayName, isOn: Binding(get: { selected }, set: { enabled in
                if payers { payerWeights[user.userID] = enabled ? "1" : nil }
                else { payeeWeights[user.userID] = enabled ? "1" : nil }
            }))
            if selected {
                HStack {
                    if custom {
                        Text("Shares")
                        TextField("Shares", text: Binding(get: {
                            (payers ? payerWeights : payeeWeights)[user.userID] ?? ""
                        }, set: { value in
                            if payers { payerWeights[user.userID] = value }
                            else { payeeWeights[user.userID] = value }
                        }))
                            .keyboardType(.numberPad)
                            .textFieldStyle(.roundedBorder)
                            .multilineTextAlignment(.trailing)
                            .frame(width: 110)
                            .accessibilityLabel("\(user.displayName) shares")
                    }
                    Spacer()
                    if let cents = (payers ? preview?.payers : preview?.payees)?[user.userID] {
                        Text(Money.string(cents: cents, currency: currency))
                            .monospacedDigit()
                            .font(.caption)
                            .foregroundStyle(.secondary)
                    }
                }
            }
        }
    }

    private func weights(_ shares: [Share]) -> [String: String] {
        shares.reduce(into: [String: UInt64]()) { totals, share in
            totals[share.userID, default: 0] += UInt64(share.shares)
        }.mapValues { String($0) }
    }

    private func seed() {
        guard !initialized else { return }
        initialized = true
        if let bill {
            description = bill.description
            amount = Money.input(cents: bill.amountCents)
            payerWeights = weights(bill.payers)
            payeeWeights = weights(bill.payees)
            customPayers = payerWeights.values.contains { $0 != "1" }
            customPayees = payeeWeights.values.contains { $0 != "1" }
        } else {
            if let first = users.first(where: { !$0.archived }) { payerWeights[first.userID] = "1" }
            payeeWeights = Dictionary(uniqueKeysWithValues: users.filter { !$0.archived }.map { ($0.userID, "1") })
        }
    }

    private func updatePreview() async {
        preview = nil
        previewError = nil
        guard initialized, !amount.isEmpty else { return }
        let key = draftKey
        do {
            let split = try await console.previewBill(amountCents: Money.parse(amount),
                payers: shares(payerWeights, custom: customPayers), payees: shares(payeeWeights, custom: customPayees), previewID: previewID)
            guard !Task.isCancelled, key == draftKey else { return }
            preview = split
        } catch {
            guard !Task.isCancelled, key == draftKey else { return }
            previewError = error.localizedDescription
        }
    }

    private func save() async {
        isSaving = true
        error = nil
        do {
            try await console.saveBill(ledgerID: ledgerID, description: description.trimmingCharacters(in: .whitespacesAndNewlines),
                amountCents: Money.parse(amount), payers: shares(payerWeights, custom: customPayers),
                payees: shares(payeeWeights, custom: customPayees), previousBillIDs: bill.map { [$0.id] } ?? [], billID: previewID)
            onSaved()
            if !embedded { dismiss() }
        } catch { self.error = error.localizedDescription }
        isSaving = false
    }
}
