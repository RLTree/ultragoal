mod evidence;
mod ladder;
mod model;
mod v2;

pub use ladder::source_candidate_ceilings;
pub use model::{
    ClaimCeiling, DimensionDisposition, DimensionEvidence, EvidenceBinding, EvidenceClass,
    FitnessDimension, ManualJourneyRow, OperatorKind, OverallDisposition,
    ProductFitnessDisposition, ProductFitnessError, PublicEntryObservation, RealWorkObservation,
    SurfaceIdentities, SurfaceIdentity, SurfaceStatus, TRUTH_LAYERS, TruthLayer,
};
use std::collections::BTreeSet;
use std::path::Path;

impl ProductFitnessDisposition {
    pub fn validate(&self, root: &Path) -> Result<(), ProductFitnessError> {
        if !matches!(
            self.schema_version.as_str(),
            "HarnessProductFitnessDisposition-v1" | "HarnessProductFitnessDisposition-v2"
        ) {
            return Err(ProductFitnessError::InvalidDisposition);
        }
        evidence::validate_digest(&self.candidate_id)?;
        evidence::validate_actor(&self.producer_actor_id)?;
        evidence::validate_actor(&self.reviewer_actor_id)?;
        if self.producer_actor_id == self.reviewer_actor_id {
            return Err(ProductFitnessError::ActorNotDisjoint);
        }
        if self.owner_role != "product-journey-reviewer"
            || self.reviewer_authority != "falsification_evidence_only"
            || self.may_raise_claim_ceiling
        {
            return Err(ProductFitnessError::ReviewerAuthorityInvalid);
        }
        validate_dimensions(self, root)?;
        validate_substitutions(self)?;
        validate_overall(self)?;
        if self.schema_version == "HarnessProductFitnessDisposition-v2" {
            v2::validate(self, root)?;
            ladder::validate(&self.truth_layer_ceilings)?;
        } else if self.operator_kind.is_some()
            || self.evidence_class.is_some()
            || self.surface_identities.is_some()
            || self.public_entry_observation.is_some()
            || self.real_work_observation.is_some()
            || self.manual_journey_row.is_some()
        {
            return Err(ProductFitnessError::InvalidDisposition);
        } else {
            ladder::validate(&self.truth_layer_ceilings)?;
        }
        Ok(())
    }
}

fn validate_dimensions(
    disposition: &ProductFitnessDisposition,
    root: &Path,
) -> Result<(), ProductFitnessError> {
    let expected = BTreeSet::from([
        FitnessDimension::Accessibility,
        FitnessDimension::CognitiveLoad,
        FitnessDimension::RecoveryBurden,
        FitnessDimension::Continuance,
        FitnessDimension::RealUseEvidence,
    ]);
    let mut observed = BTreeSet::new();
    for dimension in &disposition.dimensions {
        if !observed.insert(dimension.dimension) {
            return Err(ProductFitnessError::DuplicateDimension);
        }
        if dimension.finding.trim().is_empty() {
            return Err(ProductFitnessError::InvalidDisposition);
        }
        evidence::validate(root, &disposition.candidate_id, &dimension.evidence)?;
    }
    if observed != expected {
        return Err(ProductFitnessError::MissingDimension);
    }
    Ok(())
}

fn validate_substitutions(
    disposition: &ProductFitnessDisposition,
) -> Result<(), ProductFitnessError> {
    let required = BTreeSet::from([
        "documentation_only".to_owned(),
        "fixture_pass".to_owned(),
        "install_success".to_owned(),
        "package_publication".to_owned(),
        "receipt_only".to_owned(),
        "reviewer_agreement".to_owned(),
        "smoke_test".to_owned(),
    ]);
    if !required.is_subset(&disposition.substitution_rejections) {
        return Err(ProductFitnessError::MissingSubstitutionRejection);
    }
    Ok(())
}

fn validate_overall(disposition: &ProductFitnessDisposition) -> Result<(), ProductFitnessError> {
    let inferred = if disposition
        .dimensions
        .iter()
        .any(|item| item.disposition == DimensionDisposition::Fail)
    {
        OverallDisposition::Fail
    } else if disposition
        .dimensions
        .iter()
        .any(|item| item.disposition == DimensionDisposition::Blocked)
    {
        OverallDisposition::Blocked
    } else {
        OverallDisposition::Pass
    };
    if disposition.overall != inferred {
        return Err(ProductFitnessError::InvalidDisposition);
    }
    if disposition.overall == OverallDisposition::Pass
        && disposition
            .dimensions
            .iter()
            .any(|item| !item.evidence.same_surface || !item.evidence.current_session)
    {
        return Err(ProductFitnessError::EvidenceStale);
    }
    Ok(())
}
