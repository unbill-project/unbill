use std::fmt;

use autosurgeon::{HydrateError, Prop, ReadDoc, Reconciler};

/// A ULID used as a typed identifier for domain entities.
///
/// Stored in the Automerge document as the canonical 26-character uppercase
/// string so that documents remain human-readable and the byte representation
/// is stable across platforms.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Ulid(ulid::Ulid);

impl Ulid {
    /// Generate a new ULID from the current time and random entropy.
    pub fn new() -> Self {
        Self(ulid::Ulid::new())
    }

    /// Parse from the canonical 26-character string representation.
    pub fn from_string(s: &str) -> Result<Self, ulid::DecodeError> {
        ulid::Ulid::from_string(s).map(Self)
    }

    /// Construct directly from a raw `u128` value.  Useful in tests for
    /// creating deterministic, human-labelled identifiers (e.g. `Ulid::from_u128(1)`
    /// for "alice", `Ulid::from_u128(2)` for "bob").
    pub fn from_u128(n: u128) -> Self {
        Self(ulid::Ulid(n))
    }
}

impl Default for Ulid {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for Ulid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

// --- serde integration ---
// Serialized as the canonical 26-character ULID string.

impl serde::Serialize for Ulid {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

impl<'de> serde::Deserialize<'de> for Ulid {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        Self::from_string(&s).map_err(serde::de::Error::custom)
    }
}

// --- autosurgeon integration ---
// Stored as a String in the Automerge document.

impl autosurgeon::Reconcile for Ulid {
    type Key<'a> = autosurgeon::reconcile::NoKey;

    fn reconcile<R: Reconciler>(&self, reconciler: R) -> Result<(), R::Error> {
        self.to_string().reconcile(reconciler)
    }
}

impl autosurgeon::Hydrate for Ulid {
    fn hydrate<'a, D: ReadDoc>(
        doc: &'a D,
        obj: &automerge::ObjId,
        prop: Prop<'a>,
    ) -> Result<Self, HydrateError> {
        let s = String::hydrate(doc, obj, prop)?;
        Self::from_string(&s).map_err(|e| HydrateError::unexpected("ULID string", e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ulid_new_is_unique() {
        let a = Ulid::new();
        let b = Ulid::new();
        assert_ne!(a, b);
    }

    #[test]
    fn test_ulid_round_trip_string() {
        let id = Ulid::new();
        let s = id.to_string();
        let parsed = Ulid::from_string(&s).unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn test_ulid_from_u128_deterministic() {
        assert_eq!(Ulid::from_u128(1), Ulid::from_u128(1));
        assert_ne!(Ulid::from_u128(1), Ulid::from_u128(2));
    }

    #[test]
    fn test_ulid_ordering_matches_u128() {
        let a = Ulid::from_u128(1);
        let b = Ulid::from_u128(2);
        assert!(a < b);
    }
}
