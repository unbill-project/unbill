import SwiftUI

// sirno:witness:unbill-license:begin
struct LicensesView: View {
    @State private var software: [LegalSoftware] = []
    @State private var query = ""
    @State private var loading = true

    private var filtered: [LegalSoftware] { software.filter { $0.matches(query) } }

    var body: some View {
        Group {
            if loading {
                ProgressView("Loading licenses…")
            } else {
                List {
                    if filtered.isEmpty {
                        Text("No matching software.").foregroundStyle(.secondary)
                    }
                    ForEach(filtered) { item in
                        NavigationLink {
                            LicenseDetailView(software: item)
                        } label: {
                            VStack(alignment: .leading, spacing: 4) {
                                Text(item.displayName)
                                Text(item.versions.joined(separator: ", "))
                                    .font(.caption).foregroundStyle(.secondary)
                                Text(item.licenseExpressions.joined(separator: " · "))
                                    .font(.caption).foregroundStyle(.secondary)
                            }
                        }
                    }
                }
            }
        }
        .navigationTitle("Open Source Licenses")
        .searchable(text: $query, prompt: "Search software")
        .task {
            let loaded = await LicenseCatalogClient.shared.software()
            guard !Task.isCancelled else { return }
            software = loaded
            loading = false
        }
    }
}

private struct LicenseDetailView: View {
    let software: LegalSoftware
    @State private var packages: [LegalPackage] = []
    @State private var loading = true

    var body: some View {
        Group {
            if loading {
                ProgressView("Loading license texts…")
            } else {
                List {
                    if packages.isEmpty {
                        Text("License information unavailable.").foregroundStyle(.secondary)
                    }
                    ForEach(packages) { package in
                        Section("\(software.displayName) \(package.version)") {
                            Text(package.licenseExpression).textSelection(.enabled)
                            if let repository = package.repository {
                                Text(repository).font(.caption).textSelection(.enabled)
                            }
                            if let source = package.source {
                                Text(source).font(.caption).textSelection(.enabled)
                            }
                            ForEach(package.documents) { document in
                                DisclosureGroup(document.title) {
                                    Text(document.text)
                                        .font(.callout.monospaced())
                                        .textSelection(.enabled)
                                        .fixedSize(horizontal: false, vertical: true)
                                        .frame(maxWidth: .infinity, alignment: .leading)
                                }
                            }
                        }
                    }
                }
            }
        }
        .navigationTitle(software.displayName)
        .task(id: software.name) {
            let loaded = await LicenseCatalogClient.shared.details(name: software.name)
            guard !Task.isCancelled else { return }
            packages = loaded
            loading = false
        }
    }
}
// sirno:witness:unbill-license:end
