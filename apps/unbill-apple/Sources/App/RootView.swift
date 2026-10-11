import SwiftUI

// Mac uses three columns; iPad uses two columns and iPhone uses explicit push navigation.
struct RootView: View {
    let console: ConsoleClient
    @Environment(\.horizontalSizeClass) private var horizontalSizeClass
    @State private var ledgerPath: [String] = []
    @State private var selectedLedgerID: String?
    @State private var openedLedger: LedgerDetail?
    @State private var openedLedgerName = "Ledger"
    @State private var sheet: AppSheet?
    @State private var listGeneration = 0
    @State private var detailGeneration = 0
    @State private var billEditor: BillEditorContext?
    @AppStorage("appearance") private var appearance = "system"

    var body: some View {
        navigation
        .preferredColorScheme(appearance == "light" ? .light : appearance == "dark" ? .dark : nil)
        .onChange(of: selectedLedgerID) { _, id in
            billEditor = nil
            if usesCompactNavigation {
                let path = id.map { [$0] } ?? []
                if ledgerPath != path { ledgerPath = path }
            }
        }
        .onChange(of: ledgerPath) { _, path in
            if usesCompactNavigation { selectedLedgerID = path.last }
        }
        .onChange(of: horizontalSizeClass) { _, _ in
            if usesCompactNavigation { ledgerPath = selectedLedgerID.map { [$0] } ?? [] }
        }
        .onOpenURL { url in
            guard url.scheme?.lowercased() == "unbill", url.host?.lowercased() == "join" else { return }
            sheet = .join(url.absoluteString)
        }
        .toolbar {
            ToolbarItem(placement: .automatic) {
                Button { sheet = .settings } label: {
                    Label("Settings", systemImage: "gearshape")
                }
            }
        }
        .sheet(item: $sheet) { destination in
            switch destination {
            case .settings: AppSettingsView(console: console)
            case .join(let url):
                JoinLedgerView(console: console, invitationURL: url) { listGeneration += 1 }
            }
        }
    }

    @ViewBuilder private var navigation: some View {
        #if targetEnvironment(macCatalyst)
        NavigationSplitView {
            ledgerList
                .navigationSplitViewColumnWidth(min: 180, ideal: 240, max: 360)
        } content: {
            ledgerContent
                .navigationSplitViewColumnWidth(min: 320, ideal: 420)
        } detail: {
            if let context = billEditor {
                AddBillView(console: console, ledgerID: context.detail.summary.id,
                    currency: context.detail.summary.currency, users: context.detail.users,
                    bill: context.bill, embedded: true, onCancel: { billEditor = nil }) {
                        billEditor = nil
                        openedLedger = nil
                        detailGeneration += 1
                        listGeneration += 1
                    }
                    .id(context.id)
                    .navigationSplitViewColumnWidth(min: 360, ideal: 460)
            } else {
                ContentUnavailableView("Select a bill", systemImage: "doc.text",
                    description: Text("Choose a bill to edit it, or add a new bill."))
            }
        }
        .navigationSplitViewStyle(.balanced)
        #else
        if usesCompactNavigation {
            NavigationStack(path: $ledgerPath) {
                ledgerList
                    .navigationDestination(for: String.self) { id in
                        LedgerDetailView(console: console, ledgerID: id, ledgerName: openedLedgerName, initialDetail: openedLedger?.summary.id == id ? openedLedger : nil)
                            .id(id)
                    }
            }
        } else {
            NavigationSplitView { ledgerList } detail: { ledgerContent }
        }
        #endif
    }

    private var usesCompactNavigation: Bool {
        #if targetEnvironment(macCatalyst)
        false
        #else
        horizontalSizeClass == .compact
        #endif
    }

    private var ledgerList: some View {
        LedgerListView(console: console, selection: $selectedLedgerID, onJoin: { sheet = .join("") }, onOpenLedger: { detail in
            openedLedger = detail
            openedLedgerName = detail.summary.name
            selectedLedgerID = detail.summary.id
        })
            .id(listGeneration)
    }

    @ViewBuilder private var ledgerContent: some View {
        if let id = selectedLedgerID {
            #if targetEnvironment(macCatalyst)
            LedgerDetailView(console: console, ledgerID: id, ledgerName: openedLedgerName, initialDetail: openedLedger?.summary.id == id ? openedLedger : nil, selectedBillID: billEditor?.bill?.id, onOpenBillEditor: { bill, detail in
                billEditor = BillEditorContext(bill: bill, detail: detail)
            }, onCloseBillEditor: { billEditor = nil })
                .id("\(id):\(detailGeneration)")
            #else
            LedgerDetailView(console: console, ledgerID: id, ledgerName: openedLedgerName, initialDetail: openedLedger?.summary.id == id ? openedLedger : nil).id(id)
            #endif
        } else {
            ContentUnavailableView("Select a ledger", systemImage: "list.bullet.rectangle",
                description: Text("Choose a ledger to see its bills and settlement."))
        }
    }

}

private struct BillEditorContext {
    let id = UUID()
    let bill: Bill?
    let detail: LedgerDetail
}

private enum AppSheet: Identifiable {
    case settings
    case join(String)
    var id: String {
        switch self {
        case .settings: return "settings"
        case .join(let url): return "join:" + url
        }
    }
}

private struct AppSettingsView: View {
    let console: ConsoleClient
    @AppStorage("appearance") private var appearance = "system"
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
                        Picker("Appearance", selection: $appearance) {
                            Text("System").tag("system")
                            Text("Light").tag("light")
                            Text("Dark").tag("dark")
                        }
                        .font(.body).foregroundStyle(.primary)
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
