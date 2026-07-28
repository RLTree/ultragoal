use serde::Deserialize;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub(super) enum ObservedAuthorityState {
    Active,
    CompatibilityRouteRetained,
    ContextOnly,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub(super) enum ReplacementState {
    Unverified,
    CandidateRequired,
    Verified,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub(super) enum ReaderWriterState {
    Active,
    Unknown,
    NoneVerified,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub(super) enum CompatibilityBehavior {
    Unverified,
    ExactRouteOnly,
    Verified,
    NotApplicable,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub(super) enum CompatibilityBoundary {
    #[serde(rename = "blocked-by-OD-008")]
    BlockedByOd008,
    ExplicitOnly,
    Adopted,
    NotApplicable,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub(super) enum EquivalenceProof {
    Missing,
    Verified,
    NotApplicable,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub(super) enum PhysicalCleanupState {
    #[serde(rename = "blocked-by-OD-009")]
    BlockedByOd009,
    Preserve,
    Authorized,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RouteTransition {
    pub compatibility_behavior: CompatibilityBehavior,
    pub compatibility_boundary: CompatibilityBoundary,
    pub replacement_state: ReplacementState,
    pub active_reader_writer_state: ReaderWriterState,
    pub observed_authority_state: ObservedAuthorityState,
    pub equivalence_proof: EquivalenceProof,
    pub physical_cleanup_state: PhysicalCleanupState,
    pub proof_refs: Vec<String>,
}
