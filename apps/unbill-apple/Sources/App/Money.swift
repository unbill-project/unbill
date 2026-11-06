import Foundation

// Cents + ISO currency code -> localized currency string.
enum Money {
    static func input(cents: Int64) -> String {
        String(cents / 100) + "." + String(format: "%02lld", cents % 100)
    }

    static func string(cents: Int64, currency: String) -> String {
        let amount = Decimal(cents) / 100
        return amount.formatted(.currency(code: currency))
    }
}

enum BillInputError: LocalizedError {
    case invalid(String)
    var errorDescription: String? {
        switch self { case .invalid(let message): return message }
    }
}
