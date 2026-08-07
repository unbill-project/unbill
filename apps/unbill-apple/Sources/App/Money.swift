import Foundation

// Cents + ISO currency code -> localized currency string.
enum Money {
    static func string(cents: Int64, currency: String) -> String {
        let amount = Decimal(cents) / 100
        return amount.formatted(.currency(code: currency))
    }
}
