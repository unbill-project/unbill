import SwiftUI

// NavigationSplitView adapts itself: sidebar + detail on iPad/Mac(Catalyst),
// collapses to a navigation stack on iPhone.
struct RootView: View {
    let console: ConsoleClient
    @State private var selectedLedgerID: String?

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
    }
}
