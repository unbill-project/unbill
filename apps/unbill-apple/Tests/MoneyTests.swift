import Foundation

@main
struct MoneyTests {
    static func main() throws {
        precondition(Money.input(cents: 0) == "0.00")
        precondition(Money.input(cents: 1001) == "10.01")
        print("Money input formatting passed")
    }
}
