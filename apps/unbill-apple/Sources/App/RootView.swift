import SwiftUI

// NavigationSplitView adapts itself: sidebar + detail on iPad/Mac(Catalyst),
// collapses to a navigation stack on iPhone.
struct RootView: View {
    let console: ConsoleClient
    @State private var selectedLedgerID: String?
    @State private var showingSettings = false

    var body: some View {
        NavigationSplitView {
            LedgerListView(console: console, selection: $selectedLedgerID)
        } detail: {
            if let id = selectedLedgerID {
                LedgerDetailView(console: console, ledgerID: id)
                    .id(id)
            } else {
                ContentUnavailableView(
                    "Select a ledger",
                    systemImage: "list.bullet.rectangle",
                    description: Text("Choose a ledger to see its bills and settlement.")
                )
            }
        }
        .toolbar {
            ToolbarItem(placement: .automatic) {
                Button { showingSettings = true } label: {
                    Label("Settings", systemImage: "gearshape")
                }
            }
        }
        .sheet(isPresented: $showingSettings) {
            AppSettingsView(console: console)
        }
    }
}

private struct AppSettingsView: View {
    let console: ConsoleClient
    private let client = clientBuildInfo()
    @State private var service: FfiBuildInfo?
    @State private var loading = true

    private var serviceLabel: String {
        #if targetEnvironment(macCatalyst) || os(macOS)
        return "Daemon"
        #else
        return "In-process service"
        #endif
    }

    var body: some View {
        NavigationStack {
            GeometryReader { geometry in
                ScrollView {
                    VStack(alignment: .leading, spacing: 4) {
                        Spacer(minLength: 24)
                        Text("Client: \(client.version) (built \(client.builtAtUtc))")
                        if let service {
                            Text("\(serviceLabel): \(service.version) (built \(service.builtAtUtc))")
                        } else {
                            Text("\(serviceLabel): \(loading ? "Loading…" : "Service version unavailable")")
                        }
                    }
                    .font(.caption2)
                    .foregroundStyle(.secondary)
                    .textSelection(.enabled)
                    .frame(maxWidth: .infinity, minHeight: max(0, geometry.size.height - 32), alignment: .bottomLeading)
                    .padding()
                }
            }
            .navigationTitle("Settings")
            .toolbar {
                ToolbarItem(placement: .confirmationAction) {
                    Button("Done") { dismiss() }
                }
            }
            .task {
                service = try? await console.serviceBuildInfo()
                loading = false
            }
        }
    }

    @Environment(\.dismiss) private var dismiss
}
