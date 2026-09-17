use std::fmt;

use autosurgeon::{HydrateError, Prop, ReadDoc, Reconciler};
use iso_currency::IntoEnumIterator;

/// An ISO 4217 currency code (e.g. `USD`, `EUR`, `JPY`).
///
/// A thin newtype over [`iso_currency::Currency`] that integrates with the
/// Automerge CRDT layer. Stored in documents as the canonical 3-letter
/// alphabetic code string.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Currency(iso_currency::Currency);

impl Currency {
    /// Every currency supported by the core's ISO currency definitions.
    pub fn all() -> impl Iterator<Item = Self> {
        iso_currency::Currency::iter().map(Self)
    }

    /// Look up a currency by its ISO 4217 alphabetic code (case-sensitive, uppercase).
    /// Returns `None` for unrecognised codes.
    pub fn from_code(code: &str) -> Option<Self> {
        iso_currency::Currency::from_code(code).map(Self)
    }

    /// The 3-letter ISO 4217 alphabetic code (e.g. `"USD"`).
    pub fn code(self) -> &'static str {
        self.0.code()
    }

    /// The full English name (e.g. `"United States dollar"`).
    pub fn name(self) -> String {
        self.0.name().to_owned()
    }
}

impl fmt::Display for Currency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0.code())
    }
}

// --- autosurgeon integration ---
// Stored as the 3-letter code string in the Automerge document.

impl autosurgeon::Reconcile for Currency {
    type Key<'a> = autosurgeon::reconcile::NoKey;

    fn reconcile<R: Reconciler>(&self, reconciler: R) -> Result<(), R::Error> {
        self.0.code().reconcile(reconciler)
    }
}

impl autosurgeon::Hydrate for Currency {
    fn hydrate<'a, D: ReadDoc>(
        doc: &'a D,
        obj: &automerge::ObjId,
        prop: Prop<'a>,
    ) -> Result<Self, HydrateError> {
        let s = String::hydrate(doc, obj, prop)?;
        iso_currency::Currency::from_code(&s)
            .map(Self)
            .ok_or_else(|| {
                HydrateError::unexpected("ISO 4217 currency code", format!("unknown code {s:?}"))
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn currency_catalog_matches_all_accepted_codes() {
        let currencies: Vec<_> = Currency::all().collect();
        let codes: std::collections::HashSet<_> = currencies.iter().map(|c| c.code()).collect();
        assert_eq!(codes.len(), currencies.len(), "duplicate currency codes");
        for a in b'A'..=b'Z' {
            for b in b'A'..=b'Z' {
                for c in b'A'..=b'Z' {
                    let code = String::from_utf8(vec![a, b, c]).unwrap();
                    assert_eq!(
                        codes.contains(code.as_str()),
                        Currency::from_code(&code).is_some(),
                        "{code}"
                    );
                }
            }
        }
    }

    #[test]
    fn test_currency_from_valid_code() {
        let usd = Currency::from_code("USD").unwrap();
        assert_eq!(usd.code(), "USD");
        assert_eq!(usd.name(), "United States dollar".to_owned());
    }

    #[test]
    fn test_currency_from_invalid_code() {
        assert!(Currency::from_code("QQQ").is_none());
        assert!(Currency::from_code("").is_none());
        assert!(Currency::from_code("usd").is_none()); // case-sensitive
    }

    #[test]
    fn test_currency_display() {
        let eur = Currency::from_code("EUR").unwrap();
        assert_eq!(eur.to_string(), "EUR");
    }

    #[test]
    fn test_currency_equality() {
        assert_eq!(Currency::from_code("GBP"), Currency::from_code("GBP"));
        assert_ne!(Currency::from_code("GBP"), Currency::from_code("USD"));
    }
}
