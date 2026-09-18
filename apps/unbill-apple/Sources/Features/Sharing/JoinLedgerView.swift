import SwiftUI

// Join a ledger from an invitation: scan a QR code or paste the URL.
struct JoinLedgerView: View {
    let console: ConsoleClient
    var onJoined: () -> Void

    @Environment(\.dismiss) private var dismiss
    @State private var url = ""
    #if !targetEnvironment(macCatalyst)
    @State private var isScanning = false
    #endif
    @State private var isJoining = false
    @State private var error: String?

    private var trimmedURL: String {
        url.trimmingCharacters(in: .whitespacesAndNewlines)
    }

    var body: some View {
        NavigationStack {
            Form {
                #if !targetEnvironment(macCatalyst)
                Section {
                    if QRScannerView.isAvailable {
                        Button {
                            isScanning = true
                        } label: {
                            Label("Scan QR Code", systemImage: "qrcode.viewfinder")
                        }
                    }
                } footer: {
                    if QRScannerView.isAvailable {
                        Text("Scan the QR code from the other device, or paste the invitation link below.")
                    } else {
                        Text("Paste the invitation link from the other device below.")
                    }
                }
                #endif

                Section {
                    TextField("unbill://join/…", text: $url, axis: .vertical)
                        .textInputAutocapitalization(.never)
                        .autocorrectionDisabled()
                        .lineLimit(1...4)
                } header: {
                    Text("Invitation Link")
                } footer: {
                    #if targetEnvironment(macCatalyst)
                    Text("Paste the invitation link from the other device to join its ledger.")
                    #endif
                }

                if isJoining {
                    Section { HStack { ProgressView(); Text("Joining…").foregroundStyle(.secondary) } }
                }
                if let error {
                    Section { Text(error).foregroundStyle(.red) }
                }
            }
            .navigationTitle("Join Ledger")
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Cancel") { dismiss() }
                }
                ToolbarItem(placement: .confirmationAction) {
                    Button("Join") { Task { await join(trimmedURL) } }
                        .disabled(trimmedURL.isEmpty || isJoining)
                }
            }
            #if !targetEnvironment(macCatalyst)
            .sheet(isPresented: $isScanning) {
                NavigationStack {
                    QRScannerView { scanned in
                        isScanning = false
                        url = scanned
                        Task { await join(scanned) }
                    }
                    .ignoresSafeArea()
                    .navigationTitle("Scan QR")
                    .toolbar {
                        ToolbarItem(placement: .cancellationAction) {
                            Button("Cancel") { isScanning = false }
                        }
                    }
                }
            }
            #endif
        }
    }

    private func join(_ link: String) async {
        let link = link.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !link.isEmpty else { return }
        isJoining = true
        error = nil
        do {
            try await console.joinLedger(url: link, label: nil)
            onJoined()
            dismiss()
        } catch {
            self.error = error.localizedDescription
        }
        isJoining = false
    }
}
