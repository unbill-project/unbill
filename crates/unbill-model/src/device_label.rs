use crate::UnbillError;
use autosurgeon::{HydrateError, Prop, ReadDoc, Reconciler};

// sirno:witness:users-and-devices:begin
/// The shared name of a device inside one ledger. Never a local alias.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct DeviceLabel(String);
impl DeviceLabel {
    pub fn new(value: String) -> Result<Self, UnbillError> {
        let value = value.trim();
        if value.is_empty() || value.chars().count() > 100 {
            return Err(UnbillError::Validation(
                "device name must contain 1 to 100 characters".into(),
            ));
        }
        Ok(Self(value.to_owned()))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl TryFrom<String> for DeviceLabel {
    type Error = UnbillError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}
impl From<DeviceLabel> for String {
    fn from(value: DeviceLabel) -> Self {
        value.0
    }
}
impl std::fmt::Display for DeviceLabel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl autosurgeon::Reconcile for DeviceLabel {
    type Key<'a> = autosurgeon::reconcile::NoKey;
    fn reconcile<R: Reconciler>(&self, reconciler: R) -> Result<(), R::Error> {
        self.0.reconcile(reconciler)
    }
}
impl autosurgeon::Hydrate for DeviceLabel {
    fn hydrate<'a, D: ReadDoc>(
        doc: &'a D,
        obj: &automerge::ObjId,
        prop: Prop<'a>,
    ) -> Result<Self, HydrateError> {
        let value = String::hydrate(doc, obj, prop)?;
        Self::new(value)
            .map_err(|error| HydrateError::unexpected("a valid device name", error.to_string()))
    }
}
// sirno:witness:users-and-devices:end
