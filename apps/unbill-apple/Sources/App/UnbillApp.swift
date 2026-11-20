import SwiftUI

@main
struct UnbillApp: App {
    // Backed by the real Rust core (unbill-ffi via UniFFI over UnbillConsole).
    @State private var startup: Result<ConsoleClient, Error>
    @AppStorage("appearance") private var appearance = "system"

    init() {
        _startup = State(initialValue: Result { try RustConsoleClient() })
    }

    var body: some Scene {
        WindowGroup {
            Group {
                switch startup {
                case .success(let console):
                    AccountGateView(console: console)
                case .failure(let error):
                    StartupFailureView(error: error) { startup = Result { try RustConsoleClient() } }
                }
            }
            .background(WindowAppearance(appearance: appearance))
        }
    }
}

private struct AccountGateView: View {
    let console: ConsoleClient
    @AppStorage("accountSession") private var sessionToken = ""
    @State private var accountName: String?
    @State private var loading = true
    @State private var name = ""
    @State private var password = ""
    @State private var oldPassword = ""
    @State private var newPassword = ""
    @State private var error: String?
    @State private var showingAccount = false

    var body: some View {
        Group {
            if loading {
                ProgressView("Opening account…")
            } else if let accountName {
                RootView(console: console)
                    .id(accountName)
                    .overlay(alignment: .bottomTrailing) {
                        Button(accountName, systemImage: "person.crop.circle") { showingAccount = true }
                            .buttonStyle(.bordered)
                            .padding()
                    }
                    .sheet(isPresented: $showingAccount) {
                        NavigationStack {
                            Form {
                                Section("Account") { Text(accountName) }
                                Section("Change password") {
                                    SecureField("Current password", text: $oldPassword)
                                    SecureField("New password", text: $newPassword)
                                    Button("Change password") { Task { await changePassword() } }
                                        .disabled(oldPassword.isEmpty || newPassword.isEmpty)
                                }
                                Section { Button("Log out") { Task { await logout() } } }
                                if let error { Text(error).foregroundStyle(.red) }
                            }
                            .navigationTitle("Account")
                            .toolbar { Button("Done") { showingAccount = false } }
                        }
                    }
            } else {
                NavigationStack {
                    Form {
                        Section("Account") {
                            TextField("Account name", text: $name)
                                .textInputAutocapitalization(.never)
                            SecureField("Password", text: $password)
                        }
                        if let error { Text(error).foregroundStyle(.red) }
                        Section {
                            Button("Log in") { Task { await authenticate(register: false) } }
                                .disabled(name.isEmpty || password.isEmpty)
                            Button("Register") { Task { await authenticate(register: true) } }
                                .disabled(name.isEmpty || password.isEmpty)
                        }
                    }
                    .navigationTitle("Unbill")
                }
            }
        }
        .task {
            guard loading else { return }
            if let current = try? await console.currentAccount() { accountName = current }
            else if !sessionToken.isEmpty,
                let resumed = try? await console.resumeAccount(token: sessionToken) { accountName = resumed }
            else { sessionToken = "" }
            loading = false
        }
    }

    private func authenticate(register: Bool) async {
        do {
            if register { try await console.registerAccount(name: name, password: password) }
            sessionToken = try await console.loginAccount(name: name, password: password)
            accountName = name
            password = ""
            error = nil
        } catch { self.error = error.localizedDescription }
    }

    private func logout() async {
        do { try await console.logoutAccount() }
        catch { self.error = error.localizedDescription }
        sessionToken = ""
        accountName = nil
        showingAccount = false
    }

    private func changePassword() async {
        do {
            try await console.changeAccountPassword(old: oldPassword, new: newPassword)
            sessionToken = ""
            accountName = nil
            showingAccount = false
            oldPassword = ""
            newPassword = ""
        } catch { self.error = error.localizedDescription }
    }
}

private struct StartupFailureView: View {
    let error: Error
    var onRetry: () -> Void
    @State private var showingLicenses = false
    @State private var showingAbout = false

    private var failure: (title: String, icon: String, message: String) {
        switch error {
        case ConsoleError.DataDirectoryInUse(let path):
            return ("Data Directory Already in Use", "lock.fill",
                "Another Unbill app or unbill-daemon is using this directory:\n\n\(path)\n\nQuit the other app or stop the daemon, then reopen Unbill.")
        case ConsoleError.Message(let message):
            return ("Unable to Open Unbill", "exclamationmark.triangle", message)
        default:
            return ("Unable to Open Unbill", "exclamationmark.triangle", error.localizedDescription)
        }
    }

    var body: some View {
        ContentUnavailableView {
            Label(failure.title, systemImage: failure.icon)
        } description: {
            Text(failure.message)
        } actions: {
            Button("Retry", action: onRetry)
                .buttonStyle(.borderedProminent)
                .controlSize(.large)
                .keyboardShortcut(.defaultAction)
            Button("Open Source Licenses") { showingLicenses = true }
            Button(AboutContent.navigationTitle) { showingAbout = true }
        }
        .sheet(isPresented: $showingAbout) {
            NavigationStack {
                AboutView()
                    .toolbar {
                        ToolbarItem(placement: .confirmationAction) {
                            Button("Done") { showingAbout = false }
                        }
                    }
            }
        }
        .sheet(isPresented: $showingLicenses) {
            NavigationStack {
                LicensesView()
                    .toolbar {
                        ToolbarItem(placement: .confirmationAction) {
                            Button("Done") { showingLicenses = false }
                        }
                    }
            }
        }
    }
}
