//! Shared input limits. UI adapters and model validation use these Garde rules.

use garde::Error;

pub const MAX_BILL_AMOUNT_CENTS: i64 = unbill_model_verified::ledger::check::MAX_AMOUNT_CENTS;
pub const MAX_BILL_DESCRIPTION_BYTES: usize = 1000;
pub const MAX_NAME_BYTES: usize = 100;
pub const MAX_DEVICE_NAME_CHARS: usize = 100;
pub const BILL_AMOUNT_ERROR: &str = "Amount must be between 0 and 20,000,000.00.";
pub const BILL_DESCRIPTION_ERROR: &str = "Description must contain 1 to 1000 UTF-8 bytes.";
pub const SHARE_WEIGHT_ERROR: &str = "Shares must be whole numbers from 1 to 4294967295.";
pub const LEDGER_NAME_ERROR: &str = "Ledger name must contain 1 to 100 UTF-8 bytes.";
pub const USER_NAME_ERROR: &str = "User name must contain 1 to 100 UTF-8 bytes.";
pub const DEVICE_NAME_ERROR: &str = "Device name must contain 1 to 100 characters.";

/// Parse the decimal input used by every bill editor, then apply Garde bounds.
pub fn parse_bill_amount_text(input: &str) -> Result<i64, String> {
    let normalized = input.trim().replace(',', ".");
    let mut parts = normalized.split('.');
    let units = parts.next().unwrap_or_default();
    if units.is_empty() || !units.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err("Enter a valid amount, such as 12.50.".to_owned());
    }
    let fraction = parts.next();
    if parts.next().is_some() {
        return Err("Enter a valid amount, such as 12.50.".to_owned());
    }
    let fractional_cents = match fraction {
        None | Some("") => 0,
        Some(digits) if digits.len() <= 2 && digits.bytes().all(|byte| byte.is_ascii_digit()) => {
            let value = digits
                .parse::<i64>()
                .map_err(|_| "Amount is too large.".to_owned())?;
            if digits.len() == 1 {
                value
                    .checked_mul(10)
                    .ok_or_else(|| "Amount is too large.".to_owned())?
            } else {
                value
            }
        }
        Some(digits) if digits.len() > 2 && digits.bytes().all(|byte| byte.is_ascii_digit()) => {
            return Err("Use at most two decimal places.".to_owned());
        }
        _ => return Err("Enter a valid amount, such as 12.50.".to_owned()),
    };
    let cents = units
        .parse::<i64>()
        .ok()
        .and_then(|whole| whole.checked_mul(100))
        .and_then(|whole_cents| whole_cents.checked_add(fractional_cents))
        .ok_or_else(|| "Amount is too large.".to_owned())?;
    bill_amount(cents)?;
    Ok(cents)
}

pub fn bill_amount(value: i64) -> Result<(), String> {
    bill_amount_rule(&value, &()).map_err(|_| BILL_AMOUNT_ERROR.to_owned())
}

pub fn bill_description(value: &str) -> Result<(), String> {
    bill_description_rule(value, &()).map_err(|_| BILL_DESCRIPTION_ERROR.to_owned())
}

pub fn share_weight(value: u32) -> Result<(), String> {
    share_weight_rule(&value, &()).map_err(|_| SHARE_WEIGHT_ERROR.to_owned())
}

pub fn ledger_name(value: &str) -> Result<(), String> {
    ledger_name_rule(value, &()).map_err(|_| LEDGER_NAME_ERROR.to_owned())
}

pub fn user_name(value: &str) -> Result<(), String> {
    user_name_rule(value, &()).map_err(|_| USER_NAME_ERROR.to_owned())
}

pub fn device_name(value: &str) -> Result<(), String> {
    device_name_rule(value, &()).map_err(|_| DEVICE_NAME_ERROR.to_owned())
}

pub(crate) fn bill_amount_rule(value: &i64, _: &()) -> Result<(), Error> {
    garde::rules::range::apply(value, (Some(0), Some(MAX_BILL_AMOUNT_CENTS)))
}

pub(crate) fn bill_description_rule(value: &str, _: &()) -> Result<(), Error> {
    garde::rules::length::simple::apply(&value, (1, MAX_BILL_DESCRIPTION_BYTES))
}

pub(crate) fn share_weight_rule(value: &u32, _: &()) -> Result<(), Error> {
    garde::rules::range::apply(value, (Some(1), None))
}

pub(crate) fn ledger_name_rule(value: &str, _: &()) -> Result<(), Error> {
    garde::rules::length::simple::apply(&value, (1, MAX_NAME_BYTES))
}

pub(crate) fn user_name_rule(value: &str, _: &()) -> Result<(), Error> {
    garde::rules::length::simple::apply(&value, (1, MAX_NAME_BYTES))
}

pub(crate) fn device_name_rule(value: &str, _: &()) -> Result<(), Error> {
    garde::rules::length::chars::apply(&value, (1, MAX_DEVICE_NAME_CHARS))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_boundaries_match_the_verified_model() {
        assert!(bill_amount(0).is_ok());
        assert!(bill_amount(MAX_BILL_AMOUNT_CENTS).is_ok());
        assert!(bill_amount(-1).is_err());
        assert!(bill_amount(MAX_BILL_AMOUNT_CENTS + 1).is_err());
        assert!(ledger_name(&"é".repeat(50)).is_ok());
        assert_eq!(
            ledger_name(&"é".repeat(51)),
            Err(LEDGER_NAME_ERROR.to_owned())
        );
        assert_eq!(user_name(""), Err(USER_NAME_ERROR.to_owned()));
        assert!(device_name(&"é".repeat(100)).is_ok());
        assert_eq!(
            device_name(&"é".repeat(101)),
            Err(DEVICE_NAME_ERROR.to_owned())
        );
        assert_eq!(bill_description(""), Err(BILL_DESCRIPTION_ERROR.to_owned()));
        assert_eq!(share_weight(0), Err(SHARE_WEIGHT_ERROR.to_owned()));
        assert_eq!(parse_bill_amount_text("12,5"), Ok(1250));
        assert_eq!(parse_bill_amount_text("0.00"), Ok(0));
        assert_eq!(
            parse_bill_amount_text("20000000.00"),
            Ok(MAX_BILL_AMOUNT_CENTS)
        );
        assert_eq!(
            parse_bill_amount_text("20000000.01"),
            Err(BILL_AMOUNT_ERROR.to_owned())
        );
        assert_eq!(
            parse_bill_amount_text("-0.01"),
            Err("Enter a valid amount, such as 12.50.".to_owned())
        );
        assert_eq!(
            parse_bill_amount_text("12.345"),
            Err("Use at most two decimal places.".to_owned())
        );
    }
}
