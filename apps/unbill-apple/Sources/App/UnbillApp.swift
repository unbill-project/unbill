import SwiftUI

@main
struct UnbillApp: App {
    // Backed by the real Rust core (unbill-ffi via UniFFI over UnbillConsole).
    private let console: ConsoleClient

    init() {
        do {
            console = try RustConsoleClient()
        } catch {
            // The console is the whole app; there is no fallback.
            fatalError("Failed to open the unbill console: \(error)")
        }
    }

    var body: some Scene {
        WindowGroup {
            RootView(console: console)
        }
    }
}
