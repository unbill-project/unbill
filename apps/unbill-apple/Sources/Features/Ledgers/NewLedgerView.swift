import SwiftUI

// Sheet for creating a ledger. Calls the console (real Rust or mock) and hands
// the created summary back to the caller to refresh its list.
struct NewLedgerView: View {
    let console: ConsoleClient
    var onCreated: (LedgerSummary) -> Void

    @Environment(\.dismiss) private var dismiss
    @State private var name = ""
    @State private var currency = "USD"
    @State private var isSaving = false
    @State private var error: String?

    // Common ISO codes; validated by the core's Currency::from_code.
    private let currencies = [
        "USD", "EUR", "GBP", "JPY", "CAD", "AUD", "CHF", "CNY", "INR", "SEK", "NOK", "NZD",
    ]

    private var trimmedName: String {
        name.trimmingCharacters(in: .whitespacesAndNewlines)
    }

    var body: some View {
        NavigationStack {
            Form {
                Section {
                    TextField("Name", text: $name)
                        .textInputAutocapitalization(.words)
                    Picker("Currency", selection: $currency) {
                        ForEach(currencies, id: \.self) { Text($0).tag($0) }
                    }
                }
                if let error {
                    Section {
                        Text(error).foregroundStyle(.red)
                    }
                }
            }
            .navigationTitle("New Ledger")
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Cancel") { dismiss() }
                }
                ToolbarItem(placement: .confirmationAction) {
                    Button("Create") { Task { await create() } }
                        .disabled(trimmedName.isEmpty || isSaving)
                }
            }
        }
    }

    private func create() async {
        isSaving = true
        error = nil
        do {
            let summary = try await console.createLedger(name: trimmedName, currency: currency)
            onCreated(summary)
            dismiss()
        } catch {
            self.error = error.localizedDescription
        }
        isSaving = false
    }
}
