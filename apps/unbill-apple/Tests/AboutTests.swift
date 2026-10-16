import Foundation

@main
struct AboutTests {
    static func main() throws {
        let url = URL(fileURLWithPath: CommandLine.arguments[1])
        let content = try AboutContent.load(from: url)
        precondition(!content.title.isEmpty)
        let ids = content.sections.map(\.id)
        precondition(Set(ids).count == ids.count)
        precondition(["local-first", "architecture", "contributors", "support"].allSatisfy(ids.contains))
        for section in content.sections {
            precondition(!section.title.isEmpty && !section.paragraphs.isEmpty)
            if let link = section.link {
                precondition(!link.label.isEmpty)
                precondition(link.url.scheme == "https" && link.url.host == "github.com")
            }
        }
        print("Shared About content decodes in Swift")
    }
}
