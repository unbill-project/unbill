import Foundation

@main
struct LicenseTests {
    static func main() {
        let software = LegalSoftware(name: "unbill", versions: ["0.0.7"],
            licenseExpressions: ["MIT OR Apache-2.0"])
        precondition(software.displayName == "Unbill")
        precondition(software.matches(" UNB "))
        precondition(software.matches(""))
        precondition(!software.matches("serde"))
        print("License search tests passed")
    }
}
