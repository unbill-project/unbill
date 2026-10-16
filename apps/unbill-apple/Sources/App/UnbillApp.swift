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
                    RootView(console: console)
                case .failure(let error):
                    StartupFailureView(error: error) { startup = Result { try RustConsoleClient() } }
                }
            }
            .background(WindowAppearance(appearance: appearance))
        }
    }
}

private struct StartupFailureView: View {
    let error: Error
    var onRetry: () -> Void
    @State private var showingLicenses = false
    @State private var showingAbout = false

    private var failure: (title: String, icon: String, message: String) {
        switch error {
        case FfiError.DataDirectoryInUse(let path):
            return ("Data Directory Already in Use", "lock.fill",
                "Another Unbill app or unbill-daemon is using this directory:\n\n\(path)\n\nQuit the other app or stop the daemon, then reopen Unbill.")
        case FfiError.Message(let message):
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
