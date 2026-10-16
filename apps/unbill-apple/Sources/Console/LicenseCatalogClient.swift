import Foundation

// Embedded metadata is independent of Console and the device service.
// Decode and copy Rust texts off the main actor.
actor LicenseCatalogClient {
    static let shared = LicenseCatalogClient()

    func software() -> [LegalSoftware] {
        licenseSoftwareSummaries().map {
            LegalSoftware(name: $0.name, versions: $0.versions,
                licenseExpressions: $0.licenseExpressions)
        }
    }

    func details(name: String) -> [LegalPackage] {
        licenseSoftwareDetails(name: name).map {
            LegalPackage(id: $0.packageId, name: $0.name, version: $0.version,
                licenseExpression: $0.licenseExpression, repository: $0.repository,
                source: $0.source, documents: $0.documents.map {
                    LegalDocument(title: $0.title, text: $0.text)
                })
        }
    }
}
