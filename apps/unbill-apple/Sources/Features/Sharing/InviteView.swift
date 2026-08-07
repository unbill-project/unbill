import SwiftUI

// Show an invitation for a ledger as a scannable QR code + shareable URL.
struct InviteView: View {
    let console: ConsoleClient
    let ledgerID: String
    let ledgerName: String

    @Environment(\.dismiss) private var dismiss
    @State private var invitationURL: String?
    @State private var error: String?

    var body: some View {
        NavigationStack {
            VStack(spacing: 20) {
                if let url = invitationURL {
                    Text("Scan to join “\(ledgerName)”")
                        .font(.headline)
                        .multilineTextAlignment(.center)

                    if let image = QRCode.image(from: url) {
                        Image(uiImage: image)
                            .interpolation(.none)
                            .resizable()
                            .scaledToFit()
                            .frame(maxWidth: 260, maxHeight: 260)
                            .padding(16)
                            .background(.white, in: RoundedRectangle(cornerRadius: 16))
                    }

                    Text(url)
                        .font(.footnote)
                        .foregroundStyle(.secondary)
                        .lineLimit(2)
                        .truncationMode(.middle)
                        .textSelection(.enabled)
                        .padding(.horizontal)

                    ShareLink(item: url) {
                        Label("Share Invitation", systemImage: "square.and.arrow.up")
                    }
                    .buttonStyle(.borderedProminent)
                } else if let error {
                    ContentUnavailableView(
                        "Couldn’t Create Invitation",
                        systemImage: "exclamationmark.triangle",
                        description: Text(error)
                    )
                } else {
                    ProgressView("Creating invitation…")
                }

                Spacer()
            }
            .padding()
            .navigationTitle("Invite")
            .toolbar {
                ToolbarItem(placement: .confirmationAction) {
                    Button("Done") { dismiss() }
                }
            }
            .task {
                do {
                    invitationURL = try await console.createInvitation(ledgerID: ledgerID)
                } catch {
                    self.error = error.localizedDescription
                }
            }
        }
    }
}
