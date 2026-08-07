import SwiftUI

// Join a ledger from an invitation: scan a QR code or paste the URL.
struct JoinLedgerView: View {
    let console: ConsoleClient
    var onJoined: () -> Void

    @Environment(\.dismiss) private var dismiss
    @State private var url = ""
    @State private var isScanning = false
    @State private var isJoining = false
    @State private var error: String?

    private var trimmedURL: String {
        url.trimmingCharacters(in: .whitespacesAndNewlines)
    }

    var body: some View {
        NavigationStack {
            Form {
                Section {
                    if QRScannerView.isAvailable {
                        Button {
                            isScanning = true
                        } label: {
                            Label("Scan QR Code", systemImage: "qrcode.viewfinder")
                        }
                    }
                } footer: {
                    Text("Scan the QR code from the other device, or paste the invitation link below.")
                }

                Section("Invitation Link") {
                    TextField("unbill://join/…", text: $url, axis: .vertical)
                        .textInputAutocapitalization(.never)
                        .autocorrectionDisabled()
                        .lineLimit(1...4)
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
