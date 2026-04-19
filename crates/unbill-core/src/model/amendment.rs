use super::bill::Bill;

/// The set of bills that are currently effective in a ledger.
///
/// A bill is effective if its ID is not referenced in the `prev` list of any
/// other bill. Bills in `prev` have been superseded by their successor.
pub struct EffectiveBills(pub Vec<Bill>);

impl EffectiveBills {
    pub fn iter(&self) -> std::slice::Iter<'_, Bill> {
        self.0.iter()
    }

    pub fn into_vec(self) -> Vec<Bill> {
        self.0
    }
}

impl IntoIterator for EffectiveBills {
    type Item = Bill;
    type IntoIter = std::vec::IntoIter<Bill>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}
