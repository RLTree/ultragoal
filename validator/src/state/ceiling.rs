use serde::Serialize;
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CeilingRelation {
    Lower,
    Equal,
    Higher,
    Incomparable,
    DifferentClaim,
}

/// A per-claim upper bound represented as a set of independently proved dimensions.
///
/// Set inclusion is the partial order. Removing dimensions lowers a ceiling; dimensions
/// are never added while deriving state.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ClaimCeiling {
    claim_id: String,
    dimensions: BTreeSet<String>,
    withheld_reasons: BTreeSet<String>,
}

impl ClaimCeiling {
    pub fn from_dimensions(
        claim_id: impl Into<String>,
        dimensions: impl IntoIterator<Item = String>,
    ) -> Self {
        Self {
            claim_id: claim_id.into(),
            dimensions: dimensions.into_iter().collect(),
            withheld_reasons: BTreeSet::new(),
        }
    }

    pub fn claim_id(&self) -> &str {
        &self.claim_id
    }

    pub fn dimensions(&self) -> &BTreeSet<String> {
        &self.dimensions
    }

    pub fn withheld_reasons(&self) -> &BTreeSet<String> {
        &self.withheld_reasons
    }

    pub fn relation(&self, other: &Self) -> CeilingRelation {
        if self.claim_id != other.claim_id {
            return CeilingRelation::DifferentClaim;
        }
        match (
            self.dimensions.is_subset(&other.dimensions),
            other.dimensions.is_subset(&self.dimensions),
        ) {
            (true, true) => CeilingRelation::Equal,
            (true, false) => CeilingRelation::Lower,
            (false, true) => CeilingRelation::Higher,
            (false, false) => CeilingRelation::Incomparable,
        }
    }

    pub(crate) fn lower(&mut self, dimensions: &BTreeSet<String>, reason: impl Into<String>) {
        self.dimensions.retain(|item| !dimensions.contains(item));
        self.withheld_reasons.insert(reason.into());
    }

    pub(crate) fn remove_all(&mut self, reason: impl Into<String>) {
        self.dimensions.clear();
        self.withheld_reasons.insert(reason.into());
    }
}
