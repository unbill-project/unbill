import Foundation

struct LegalSoftware: Identifiable, Sendable {
    let name: String
    let versions: [String]
    let licenseExpressions: [String]
    var id: String { name }
    var displayName: String { name == "unbill" ? "Unbill" : name }

    func matches(_ query: String) -> Bool {
        let trimmed = query.trimmingCharacters(in: .whitespacesAndNewlines)
        return trimmed.isEmpty || name.localizedCaseInsensitiveContains(trimmed)
    }
}

struct LegalDocument: Identifiable, Sendable {
    let title: String
    let text: String
    var id: String { title }
}

struct LegalPackage: Identifiable, Sendable {
    let id: String
    let name: String
    let version: String
    let licenseExpression: String
    let repository: String?
    let source: String?
    let documents: [LegalDocument]
}
