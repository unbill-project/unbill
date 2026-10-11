import SwiftUI

// Sheet for adding a person to a ledger — either import someone already known
// on this device (from other ledgers) or create a brand-new named person.
struct AddUserView: View {
    let console: ConsoleClient
    let ledgerID: String
    let existingUserIDs: Set<String>
    var onSaved: () -> Void

    @Environment(\.dismiss) private var dismiss
    @State private var name = ""
    @State private var known: [User] = []
    @State private var isLoadingKnown = true
    @State private var knownLoadError: String?
    @State private var isSaving = false
    @State private var error: String?

    private var trimmedName: String {
        name.trimmingCharacters(in: .whitespacesAndNewlines)
    }

    // Known users not already in this ledger.
    private var importable: [User] {
        known.filter { !existingUserIDs.contains($0.userID) }
            .sorted { $0.displayName.localizedStandardCompare($1.displayName) == .orderedAscending }
    }

    var body: some View {
        NavigationStack {
            Form {
                Section("New Person") {
                    TextField("Name", text: $name)
                        .textInputAutocapitalization(.words)
                }

                Section("From Other Ledgers") {
                    if isLoadingKnown {
                        ProgressView("Loading people…")
                    } else if let knownLoadError {
                        Text(knownLoadError).foregroundStyle(.red)
                        Button("Retry") { Task { await loadKnownUsers() } }
                            .disabled(isSaving)
                    } else if importable.isEmpty {
                        Text("No people from other ledgers to import.")
                            .foregroundStyle(.secondary)
                    } else {
                        ForEach(importable) { user in
                            HStack {
                                Text(user.displayName)
                                Spacer()
                                Button("Import") {
                                    Task { await importExisting(user) }
                                }
                                .buttonStyle(.bordered)
                                .disabled(isSaving)
                                .accessibilityLabel("Import \(user.displayName)")
                            }
                        }
                    }
                }

                if let error {
                    Section { Text(error).foregroundStyle(.red) }
                }
            }
            .navigationTitle("Add Person")
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Cancel") { dismiss() }
                        .disabled(isSaving)
                }
                ToolbarItem(placement: .confirmationAction) {
                    Button("Create") { Task { await createNew() } }
                        .disabled(trimmedName.isEmpty || isSaving)
                }
            }
            .task { await loadKnownUsers() }
            .refreshable { await loadKnownUsers() }
        }
        .interactiveDismissDisabled(isSaving)
    }

    private func loadKnownUsers() async {
        isLoadingKnown = true
        knownLoadError = nil
        defer { isLoadingKnown = false }
        do {
            known = try await console.knownUsers()
        } catch {
            known = []
            knownLoadError = error.localizedDescription
        }
    }

    private func createNew() async {
        guard !isSaving, !trimmedName.isEmpty else { return }
        isSaving = true
        error = nil
        do {
            try await console.createUser(ledgerID: ledgerID, displayName: trimmedName)
            onSaved()
            dismiss()
        } catch {
            self.error = error.localizedDescription
        }
        isSaving = false
    }

    private func importExisting(_ user: User) async {
        guard !isSaving else { return }
        isSaving = true
        error = nil
        do {
            try await console.addUser(ledgerID: ledgerID, userID: user.userID)
            onSaved()
            dismiss()
        } catch {
            self.error = error.localizedDescription
        }
        isSaving = false
    }
}
