import Foundation

// The generated UniFFI functions call the same Garde rules used by NewBill,
// NewLedger, NewUserName, Share, and DeviceLabel in Rust.
enum InputValidation {
    private static func detail(_ error: Error) -> String {
        if case ConsoleError.Message(let text) = error { return text }
        return error.localizedDescription
    }

    private static func message(_ check: () throws -> Void) -> String? {
        do {
            try check()
            return nil
        } catch {
            return detail(error)
        }
    }

    static func billAmount(_ cents: Int64) -> String? {
        message { try validateBillAmount(amountCents: cents) }
    }

    static func parseBillAmount(_ text: String) throws -> Int64 {
        do {
            return try parseBillAmountText(input: text)
        } catch {
            throw BillInputError.invalid(detail(error))
        }
    }

    static func billDescription(_ value: String) -> String? {
        message { try validateBillDescription(description: value) }
    }

    static func shareWeight(_ value: UInt32) -> String? {
        message { try validateShareWeight(shares: value) }
    }

    static func parseShareWeight(_ text: String) throws -> UInt32 {
        guard let value = UInt32(text) else {
            throw BillInputError.invalid(shareWeight(0) ?? "Invalid share weight.")
        }
        if let error = shareWeight(value) { throw BillInputError.invalid(error) }
        return value
    }

    static func ledgerName(_ value: String) -> String? {
        message { try validateLedgerName(name: value) }
    }

    static func userName(_ value: String) -> String? {
        message { try validateUserName(name: value) }
    }

    static func deviceName(_ value: String) -> String? {
        message { try validateDeviceName(name: value) }
    }
}
