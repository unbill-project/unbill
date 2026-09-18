import SwiftUI

@main
struct UnbillApp: App {
    // Backed by the real Rust core (unbill-ffi via UniFFI over UnbillConsole).
    private let startup: Result<ConsoleClient, Error>

    init() {
        startup = Result { try RustConsoleClient() }
    }

    var body: some Scene {
        WindowGroup {
            switch startup {
            case .success(let console):
                RootView(console: console)
            case .failure(let error):
                StartupFailureView(error: error)
            }
        }
    }
}

private struct StartupFailureView: View {
    let error: Error

    var body: some View {
        switch error {
        case FfiError.DataDirectoryInUse(let path):
            ContentUnavailableView(
                "Data Directory Already in Use",
                systemImage: "lock.fill",
                description: Text("Another Unbill app or unbill-daemon is using this directory:\n\n\(path)\n\nQuit the other app or stop the daemon, then reopen Unbill.")
            )
        case FfiError.Message(let message):
            ContentUnavailableView("Unable to Open Unbill", systemImage: "exclamationmark.triangle", description: Text(message))
        default:
            ContentUnavailableView("Unable to Open Unbill", systemImage: "exclamationmark.triangle", description: Text(error.localizedDescription))
        }
    }
}
