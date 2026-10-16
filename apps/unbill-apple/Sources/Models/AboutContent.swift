import Foundation

// sirno:witness:about-page:begin
struct AboutContent: Decodable {
    let title: String
    let name: String
    let tagline: String
    let summary: String
    let principles: [String]
    let sections: [AboutSection]

    // XcodeGen bundles the canonical Rust resource, not an Apple copy.
    static let bundled: Result<AboutContent, Error> = Result {
        guard let url = Bundle.main.url(forResource: "content", withExtension: "json") else {
            throw CocoaError(.fileNoSuchFile)
        }
        return try load(from: url)
    }

    static func load(from url: URL) throws -> AboutContent {
        try JSONDecoder().decode(AboutContent.self, from: Data(contentsOf: url))
    }

    static var navigationTitle: String { (try? bundled.get().title) ?? "About" }
}

struct AboutSection: Decodable, Identifiable {
    let id: String
    let title: String
    let paragraphs: [String]
    let link: AboutLink?
}

struct AboutLink: Decodable {
    let label: String
    let url: URL
}
// sirno:witness:about-page:end
