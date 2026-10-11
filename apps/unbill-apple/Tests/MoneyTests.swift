import Foundation

@main
struct MoneyTests {
    static func main() throws {
        let valid: [(String, Int64)] = [("12.50", 1250), ("12,5", 1250), (" 0.01 ", 1), ("21474836.47", 2147483647)]
        for (text, expected) in valid {
            let parsed = try Money.parse(text)
            precondition(parsed == expected)
        }
        for text in ["", "-1", "0", "1.001", "1e3", "NaN", "1.2.3", "21474836.48", "999999999999999999999999"] {
            do {
                _ = try Money.parse(text)
                fatalError("Invalid amount accepted: \(text)")
            } catch BillInputError.invalid { }
        }
        precondition(Money.input(cents: 1001) == "10.01")
        print("Money input validation passed")
    }
}
