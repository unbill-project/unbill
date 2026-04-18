use std::fmt;
use std::str::FromStr;

use autosurgeon::{HydrateError, Prop, ReadDoc, Reconciler};

/// The iroh EndpointId of a device — a 32-byte Ed25519 public key.
///
/// Stored in the Automerge document as the canonical hex string
/// produced by `iroh::EndpointId`'s `Display` impl.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NodeId(iroh::EndpointId);

impl NodeId {
    pub fn from_node_id(id: iroh::EndpointId) -> Self {
        Self(id)
    }

    pub fn as_node_id(self) -> iroh::EndpointId {
        self.0
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl FromStr for NodeId {
    type Err = iroh::KeyParsingError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        iroh::EndpointId::from_str(s).map(Self)
    }
}

/// Construct a deterministic `NodeId` from a single seed byte.  Only available
/// in test builds — derives a valid Ed25519 key from `[seed; 32]`.
#[cfg(test)]
impl NodeId {
    pub fn from_seed(seed: u8) -> Self {
        let secret = iroh::SecretKey::from([seed; 32]);
        Self(secret.public())
    }
}

// --- autosurgeon integration ---
// Stored as the iroh EndpointId string in the Automerge document.

impl serde::Serialize for NodeId {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

impl<'de> serde::Deserialize<'de> for NodeId {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        s.parse::<Self>().map_err(serde::de::Error::custom)
    }
}

impl autosurgeon::Reconcile for NodeId {
    type Key<'a> = autosurgeon::reconcile::NoKey;

    fn reconcile<R: Reconciler>(&self, reconciler: R) -> Result<(), R::Error> {
        self.to_string().reconcile(reconciler)
    }
}

impl autosurgeon::Hydrate for NodeId {
    fn hydrate<'a, D: ReadDoc>(
        doc: &'a D,
        obj: &automerge::ObjId,
        prop: Prop<'a>,
    ) -> Result<Self, HydrateError> {
        let s = String::hydrate(doc, obj, prop)?;
        s.parse::<Self>()
            .map_err(|e| HydrateError::unexpected("iroh EndpointId string", e.to_string()))
    }
}
