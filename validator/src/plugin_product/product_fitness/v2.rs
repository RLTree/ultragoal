use super::evidence;
use super::model::{
    ClaimCeiling, DimensionDisposition, EvidenceClass, OperatorKind, ProductFitnessDisposition,
    ProductFitnessError, SurfaceIdentities, SurfaceStatus, TruthLayer,
};
use std::collections::BTreeSet;
use std::path::Path;

pub(super) fn validate(
    disposition: &ProductFitnessDisposition,
    root: &Path,
) -> Result<(), ProductFitnessError> {
    let operator = disposition
        .operator_kind
        .ok_or(ProductFitnessError::MissingV2Field)?;
    let class = disposition
        .evidence_class
        .ok_or(ProductFitnessError::MissingV2Field)?;
    let surfaces = disposition
        .surface_identities
        .as_ref()
        .ok_or(ProductFitnessError::MissingV2Field)?;
    if !disposition
        .substitution_rejections
        .contains("legacy_dogfood_receipt")
    {
        return Err(ProductFitnessError::MissingSubstitutionRejection);
    }
    validate_operator_class(operator, class, disposition)?;
    validate_surfaces(root, &disposition.candidate_id, surfaces)?;
    validate_entry(disposition, root)?;
    validate_work(disposition, root)?;
    validate_journey(disposition)?;
    validate_ceiling(disposition, class, surfaces)
}

fn validate_operator_class(
    operator: OperatorKind,
    class: EvidenceClass,
    disposition: &ProductFitnessDisposition,
) -> Result<(), ProductFitnessError> {
    if matches!(
        class,
        EvidenceClass::HumanUse | EvidenceClass::RepeatedHumanUse
    ) && operator != OperatorKind::Human
    {
        return Err(ProductFitnessError::AgentUseRelabeledHumanUse);
    }
    if class == EvidenceClass::AgentUse && operator == OperatorKind::Human {
        return Err(ProductFitnessError::InvalidObservation);
    }
    let continuance = disposition
        .dimensions
        .iter()
        .find(|item| item.dimension == super::model::FitnessDimension::Continuance)
        .map(|item| item.disposition);
    if continuance == Some(DimensionDisposition::Pass) {
        if class == EvidenceClass::AgentUse {
            return Err(ProductFitnessError::AgentUseRelabeledContinuance);
        }
        if class != EvidenceClass::RepeatedHumanUse {
            return Err(ProductFitnessError::ContinuanceRequiresRepeatedHumanUse);
        }
    }
    Ok(())
}

fn validate_surfaces(
    root: &Path,
    candidate_id: &str,
    surfaces: &SurfaceIdentities,
) -> Result<(), ProductFitnessError> {
    let items = [
        (&surfaces.source, TruthLayer::Source),
        (&surfaces.package, TruthLayer::Package),
        (&surfaces.marketplace, TruthLayer::Marketplace),
        (&surfaces.install, TruthLayer::Install),
        (&surfaces.cache, TruthLayer::Cache),
        (&surfaces.app_registry, TruthLayer::AppRegistry),
        (&surfaces.discovery, TruthLayer::Discovery),
        (&surfaces.runtime, TruthLayer::Runtime),
        (&surfaces.journey, TruthLayer::Journey),
    ];
    let mut identities = BTreeSet::new();
    let mut paths = BTreeSet::new();
    let mut bindings = BTreeSet::new();
    for (surface, _) in items {
        match surface.status {
            SurfaceStatus::Withheld => {
                if surface.identity.is_some() || surface.evidence.is_some() {
                    return Err(ProductFitnessError::InvalidObservation);
                }
            }
            SurfaceStatus::Observed => {
                let identity = surface
                    .identity
                    .as_deref()
                    .filter(|value| !value.trim().is_empty())
                    .ok_or(ProductFitnessError::MissingObservation)?;
                if !identities.insert(identity.to_owned()) {
                    return Err(ProductFitnessError::WrongSurface);
                }
                let binding = surface
                    .evidence
                    .as_ref()
                    .ok_or(ProductFitnessError::MissingObservation)?;
                evidence::validate(root, candidate_id, binding)?;
                if !binding.same_surface {
                    return Err(ProductFitnessError::WrongSurface);
                }
                if !binding.current_session {
                    return Err(ProductFitnessError::EvidenceStale);
                }
                if !paths.insert(binding.path.clone()) || !bindings.insert(binding.clone()) {
                    return Err(ProductFitnessError::WrongSurface);
                }
            }
        }
    }
    Ok(())
}

fn validate_entry(
    disposition: &ProductFitnessDisposition,
    root: &Path,
) -> Result<(), ProductFitnessError> {
    let entry = disposition
        .public_entry_observation
        .as_ref()
        .ok_or(ProductFitnessError::MissingV2Field)?;
    if entry.surface_id != "PS-ENTRY" || entry.route != "harness-ultragoal" {
        return Err(ProductFitnessError::WrongSurface);
    }
    if entry.bypass_attempted && !entry.bypass_rejected {
        return Err(ProductFitnessError::BypassAttempt);
    }
    validate_observation_evidence(root, &disposition.candidate_id, &entry.evidence)
}

fn validate_work(
    disposition: &ProductFitnessDisposition,
    root: &Path,
) -> Result<(), ProductFitnessError> {
    let work = disposition
        .real_work_observation
        .as_ref()
        .ok_or(ProductFitnessError::MissingV2Field)?;
    evidence::validate_digest(&work.repository_identity)
        .map_err(|_| ProductFitnessError::InvalidRepository)?;
    if work.repository_identity != work.repository_evidence.sha256 {
        return Err(ProductFitnessError::InvalidRepository);
    }
    validate_observation_evidence(root, &disposition.candidate_id, &work.repository_evidence)?;
    validate_observation_evidence(root, &disposition.candidate_id, &work.evidence)?;
    if work.task_id.trim().is_empty()
        || work.task.trim().is_empty()
        || work.useful_outcome.trim().is_empty()
    {
        return Err(ProductFitnessError::InvalidTask);
    }
    Ok(())
}

fn validate_observation_evidence(
    root: &Path,
    candidate_id: &str,
    binding: &super::model::EvidenceBinding,
) -> Result<(), ProductFitnessError> {
    evidence::validate(root, candidate_id, binding)?;
    if !binding.same_surface {
        return Err(ProductFitnessError::WrongSurface);
    }
    if !binding.current_session {
        return Err(ProductFitnessError::EvidenceStale);
    }
    Ok(())
}

fn validate_journey(disposition: &ProductFitnessDisposition) -> Result<(), ProductFitnessError> {
    let row = disposition
        .manual_journey_row
        .as_ref()
        .ok_or(ProductFitnessError::MissingV2Field)?;
    if row.failure.trim().is_empty()
        || row.diagnosis.trim().is_empty()
        || row.recovery_outcome.trim().is_empty()
        || row.repeat_use_outcome.trim().is_empty()
    {
        return Err(ProductFitnessError::MissingObservation);
    }
    Ok(())
}

fn validate_ceiling(
    disposition: &ProductFitnessDisposition,
    class: EvidenceClass,
    surfaces: &SurfaceIdentities,
) -> Result<(), ProductFitnessError> {
    for layer in super::model::TRUTH_LAYERS {
        let ceiling = disposition
            .truth_layer_ceilings
            .get(&layer)
            .ok_or(ProductFitnessError::MissingTruthLayer)?;
        if *ceiling != ClaimCeiling::LiveSameSurfaceProven {
            continue;
        }
        if !supports_live_layer(layer, class) {
            return Err(ProductFitnessError::UnsupportedClaimCeiling);
        }
        let observed = match layer {
            TruthLayer::Source => surfaces.source.status,
            TruthLayer::Package => surfaces.package.status,
            TruthLayer::Marketplace => surfaces.marketplace.status,
            TruthLayer::Install => surfaces.install.status,
            TruthLayer::Cache => surfaces.cache.status,
            TruthLayer::AppRegistry => surfaces.app_registry.status,
            TruthLayer::PluginsUi => return Err(ProductFitnessError::UnsupportedClaimCeiling),
            TruthLayer::Discovery => surfaces.discovery.status,
            TruthLayer::Runtime => surfaces.runtime.status,
            TruthLayer::Journey => surfaces.journey.status,
        };
        if observed != SurfaceStatus::Observed {
            return Err(ProductFitnessError::MissingObservation);
        }
        if class == EvidenceClass::AgentUse && layer == TruthLayer::Journey {
            return Err(ProductFitnessError::UnsupportedClaimCeiling);
        }
    }
    Ok(())
}

fn supports_live_layer(layer: TruthLayer, class: EvidenceClass) -> bool {
    match class {
        EvidenceClass::Intent | EvidenceClass::Research | EvidenceClass::Prototype => false,
        EvidenceClass::Source => layer == TruthLayer::Source,
        EvidenceClass::Package => matches!(layer, TruthLayer::Source | TruthLayer::Package),
        EvidenceClass::Installed => {
            matches!(
                layer,
                TruthLayer::Source
                    | TruthLayer::Package
                    | TruthLayer::Marketplace
                    | TruthLayer::Install
            )
        }
        EvidenceClass::Runtime | EvidenceClass::AgentUse => matches!(
            layer,
            TruthLayer::Source
                | TruthLayer::Package
                | TruthLayer::Marketplace
                | TruthLayer::Install
                | TruthLayer::Cache
                | TruthLayer::AppRegistry
                | TruthLayer::Discovery
                | TruthLayer::Runtime
        ),
        EvidenceClass::HumanUse | EvidenceClass::RepeatedHumanUse => matches!(
            layer,
            TruthLayer::Source
                | TruthLayer::Package
                | TruthLayer::Marketplace
                | TruthLayer::Install
                | TruthLayer::Cache
                | TruthLayer::AppRegistry
                | TruthLayer::Discovery
                | TruthLayer::Runtime
                | TruthLayer::Journey
        ),
    }
}
