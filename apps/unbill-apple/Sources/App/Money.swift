import Foundation

// Cents + ISO currency code -> localized currency string.
enum Money {
    static func input(cents: Int64) -> String {
        String(cents / 100) + "." + String(format: "%02lld", cents % 100)
    }

    static func parse(_ text: String) throws -> Int64 {
        let raw = text.trimmingCharacters(in: .whitespacesAndNewlines).replacingOccurrences(of: ",", with: ".")
        let parts = raw.split(separator: ".", omittingEmptySubsequences: false)
        guard parts.count <= 2, let whole = parts.first, !whole.isEmpty,
              whole.utf8.allSatisfy({ $0 >= 48 && $0 <= 57 }),
              let units = Int64(whole) else {
            throw BillInputError.invalid("Enter a valid amount, such as 12.50.")
        }
        let fraction = parts.count == 2 ? String(parts[1]) : ""
        guard fraction.count <= 2, fraction.utf8.allSatisfy({ $0 >= 48 && $0 <= 57 }) else {
            throw BillInputError.invalid("Use at most two decimal places.")
        }
        let (base, overflow) = units.multipliedReportingOverflow(by: 100)
        let (cents, additionOverflow) = base.addingReportingOverflow(Int64(fraction.padding(toLength: 2, withPad: "0", startingAt: 0)) ?? 0)
        guard !overflow, !additionOverflow, cents > 0, cents <= Int64(Int32.max) else {
            throw BillInputError.invalid("Amount must be positive and no greater than 21474836.47.")
        }
        return cents
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
