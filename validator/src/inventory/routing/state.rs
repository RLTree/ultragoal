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

impl RouteTransition {
    pub fn claims_demoted(&self) -> bool {
        matches!(
            self.observed_authority_state,
            ObservedAuthorityState::ContextOnly
        )
    }

    pub fn requests_retained_compatibility_route(&self) -> bool {
        self.compatibility_behavior == CompatibilityBehavior::ExactRouteOnly
            && self.compatibility_boundary == CompatibilityBoundary::ExplicitOnly
            && self.replacement_state == ReplacementState::CandidateRequired
            && self.active_reader_writer_state == ReaderWriterState::Active
            && self.observed_authority_state == ObservedAuthorityState::CompatibilityRouteRetained
            && self.equivalence_proof == EquivalenceProof::Missing
            && self.physical_cleanup_state == PhysicalCleanupState::Preserve
    }

    pub fn claims_verified_fact(&self) -> bool {
        self.replacement_state == ReplacementState::Verified
            || self.active_reader_writer_state == ReaderWriterState::NoneVerified
            || self.compatibility_behavior == CompatibilityBehavior::Verified
            || self.compatibility_boundary == CompatibilityBoundary::Adopted
            || self.equivalence_proof == EquivalenceProof::Verified
            || self.claims_demoted()
    }

    fn has_retained_compatibility_marker(&self) -> bool {
        self.compatibility_behavior == CompatibilityBehavior::ExactRouteOnly
            || self.compatibility_boundary == CompatibilityBoundary::ExplicitOnly
            || self.replacement_state == ReplacementState::CandidateRequired
            || self.observed_authority_state == ObservedAuthorityState::CompatibilityRouteRetained
    }

    fn transition_complete(&self) -> bool {
        self.replacement_state == ReplacementState::Verified
            && self.active_reader_writer_state == ReaderWriterState::NoneVerified
            && matches!(
                self.compatibility_behavior,
                CompatibilityBehavior::Verified | CompatibilityBehavior::NotApplicable
            )
            && matches!(
                self.compatibility_boundary,
                CompatibilityBoundary::Adopted | CompatibilityBoundary::NotApplicable
            )
            && self.equivalence_proof == EquivalenceProof::Verified
    }

    pub fn verifies_agent_context_transition(&self) -> bool {
        self.compatibility_behavior == CompatibilityBehavior::NotApplicable
            && self.compatibility_boundary == CompatibilityBoundary::Adopted
            && self.replacement_state == ReplacementState::CandidateRequired
            && self.active_reader_writer_state == ReaderWriterState::NoneVerified
            && self.observed_authority_state == ObservedAuthorityState::ContextOnly
            && self.equivalence_proof == EquivalenceProof::NotApplicable
            && self.physical_cleanup_state == PhysicalCleanupState::Preserve
    }

    fn requests_agent_context_transition(&self) -> bool {
        self.replacement_state == ReplacementState::CandidateRequired
            && self.active_reader_writer_state == ReaderWriterState::NoneVerified
            && self.observed_authority_state == ObservedAuthorityState::ContextOnly
    }

    pub fn validate(
        &self,
        destructive_cleanup_authorized: bool,
        exact_matcher: bool,
        compatibility_witness: bool,
        retirement_witness: bool,
        safe_proof_refs: bool,
    ) -> Result<(), &'static str> {
        if self.physical_cleanup_state == PhysicalCleanupState::Authorized
            && !destructive_cleanup_authorized
        {
            return Err("physical cleanup is blocked by OD-009");
        }
        if self.proof_refs.len() > 32 || !safe_proof_refs {
            return Err("route proof references are invalid");
        }
        if self.requests_agent_context_transition() {
            return if exact_matcher
                && retirement_witness
                && !self.proof_refs.is_empty()
                && self.verifies_agent_context_transition()
            {
                Ok(())
            } else {
                Err("agent context route lacks exact compiled proof")
            };
        }
        if self.requests_retained_compatibility_route() {
            return if exact_matcher && compatibility_witness && !self.proof_refs.is_empty() {
                Ok(())
            } else {
                Err("retained compatibility route lacks exact compiled proof")
            };
        }
        if self.has_retained_compatibility_marker() {
            return Err("retained compatibility route state is incomplete");
        }
        if self.claims_verified_fact()
            && (!exact_matcher
                || !retirement_witness
                || self.proof_refs.is_empty()
                || !self.transition_complete())
        {
            return Err("verified route transition lacks exact compiled proof");
        }
        if self.claims_demoted() && !self.transition_complete() {
            return Err("demoted route transition is incomplete");
        }
        Ok(())
    }
}
