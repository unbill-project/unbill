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
    @State private var isSaving = false
    @State private var error: String?

    private var trimmedName: String {
        name.trimmingCharacters(in: .whitespacesAndNewlines)
    }

    // Known users not already in this ledger.
    private var importable: [User] {
        known.filter { !existingUserIDs.contains($0.userID) }
    }

    var body: some View {
        NavigationStack {
            Form {
                Section("New Person") {
                    TextField("Name", text: $name)
                        .textInputAutocapitalization(.words)
                }

                if !importable.isEmpty {
                    Section("From Other Ledgers") {
                        ForEach(importable) { user in
                            Button {
                                Task { await importExisting(user) }
                            } label: {
                                HStack {
                                    Label(user.displayName, systemImage: "person.crop.circle.badge.plus")
                                        .foregroundStyle(.primary)
                                    Spacer()
                                }
                            }
                            .disabled(isSaving)
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
                }
                ToolbarItem(placement: .confirmationAction) {
                    Button("Create") { Task { await createNew() } }
                        .disabled(trimmedName.isEmpty || isSaving)
                }
            }
            .task {
                do { known = try await console.knownUsers() } catch { /* import section just stays hidden */ }
            }
        }
    }

    private func createNew() async {
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
