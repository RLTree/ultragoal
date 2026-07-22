use super::*;
use crate::repository_fit::LocalStateDisposition;

pub(crate) fn plan_projection(
    plan: &FitPlan,
    observed_modes: &BTreeMap<String, Option<u32>>,
    desired_modes: &BTreeMap<String, u32>,
) -> Result<PlanProjection, FitAdapterError> {
    let authorization = PlanAuthorization::new(
        plan.context_id.clone(),
        plan.candidate_id.clone(),
        plan.plan_sha256.clone(),
    )
    .map_err(kernel_error)?;
    let authorization_sha256 = digest(
        &serde_json::to_vec(&authorization)
            .map_err(|_| adapter_error(AdapterErrorId::ProjectionFailed))?,
    );
    Ok(PlanProjection {
        plan_sha256: plan.plan_sha256.clone(),
        authorization_sha256,
        local_state: plan.local_state.as_ref().map(local_state_projection),
        checks: plan
            .checks
            .iter()
            .map(|check| CheckProjection {
                path: check.path.as_str().to_owned(),
                expected: expected_projection(&check.expected),
                desired_sha256: check.desired_sha256.clone(),
                expected_unix_mode: observed_modes.get(check.path.as_str()).copied().flatten(),
                desired_unix_mode: desired_modes[check.path.as_str()],
                provenance: provenance_projection(&check.provenance),
                prior_proof_sha256: check.prior_proof_sha256.clone(),
            })
            .collect(),
        mutations: plan
            .mutations
            .iter()
            .map(|mutation| MutationProjection {
                path: mutation.path.as_str().to_owned(),
                expected: expected_projection(&mutation.expected),
                replacement_sha256: mutation.replacement_sha256(),
                replacement_byte_length: mutation.replacement.len(),
                replacement_unix_mode: desired_modes[mutation.path.as_str()],
                ownership: ownership_name(mutation.ownership).to_owned(),
                provenance: provenance_projection(&mutation.provenance),
                prior_proof_sha256: mutation.prior_proof_sha256.clone(),
                rollback: RollbackEntryProjection {
                    disposition: if mutation.prior.is_some() {
                        "restore_exact"
                    } else {
                        "restore_absent"
                    }
                    .to_owned(),
                    sha256: mutation.prior.as_deref().map(digest),
                    byte_length: mutation.prior.as_ref().map_or(0, Vec::len),
                    unix_mode: observed_modes
                        .get(mutation.path.as_str())
                        .copied()
                        .flatten(),
                },
            })
            .collect(),
        conflicts: plan
            .conflicts
            .iter()
            .map(conflict_projection)
            .collect::<Result<Vec<_>, _>>()?,
        rollback_mutation_count: plan.rollback.mutation_count,
    })
}

pub(crate) fn local_state_projection(state: &LocalStatePlan) -> LocalStatePolicyProjection {
    LocalStatePolicyProjection {
        path: state.path.as_str().to_owned(),
        required_rule: state.required_rule.clone(),
        disposition: match state.disposition {
            LocalStateDisposition::Missing => "missing",
            LocalStateDisposition::NeedsUpdate => "needs_update",
            LocalStateDisposition::AlreadyIgnored => "already_ignored",
        }
        .to_owned(),
        observed_sha256: state.prior.as_deref().map(crate::repository_fit::digest),
        observed_unix_mode: state.observed_mode,
        desired_unix_mode: state.desired_mode,
        desired_sha256: state.desired_sha256(),
        replacement_sha256: state.desired_sha256(),
        replacement_byte_length: state.replacement.len(),
        mutation_required: state.mutation.is_some(),
    }
}

pub(crate) fn conflict_projection(
    conflict: &crate::repository_fit::FitConflict,
) -> Result<ConflictProjection, FitAdapterError> {
    let value = serde_json::to_value(conflict)
        .map_err(|_| adapter_error(AdapterErrorId::ProjectionFailed))?;
    let object = value
        .as_object()
        .ok_or_else(|| adapter_error(AdapterErrorId::ProjectionFailed))?;
    let path = object
        .get("path")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| adapter_error(AdapterErrorId::ProjectionFailed))?;
    let observed_sha256 = object
        .get("observed_sha256")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| adapter_error(AdapterErrorId::ProjectionFailed))?;
    let ownership = object
        .get("ownership")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| adapter_error(AdapterErrorId::ProjectionFailed))?;
    let provenance = object
        .get("provenance")
        .and_then(serde_json::Value::as_object)
        .ok_or_else(|| adapter_error(AdapterErrorId::ProjectionFailed))?;
    let source = provenance
        .get("source")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| adapter_error(AdapterErrorId::ProjectionFailed))?;
    let field = |name: &str| {
        provenance
            .get(name)
            .and_then(serde_json::Value::as_str)
            .map(ToOwned::to_owned)
    };
    Ok(ConflictProjection {
        path: path.to_owned(),
        observed_sha256: observed_sha256.to_owned(),
        ownership: ownership.replace('-', "_"),
        provenance: ProvenanceProjection {
            source: source.replace('-', "_"),
            context_id: field("context_id"),
            candidate_id: field("candidate_id"),
            authority_sha256: field("authority_sha256"),
            row_sha256: field("row_sha256"),
        },
        prior_proof_sha256: object
            .get("prior_proof_sha256")
            .and_then(serde_json::Value::as_str)
            .map(ToOwned::to_owned),
    })
}

pub(crate) fn provenance_projection(provenance: &OwnershipProvenance) -> ProvenanceProjection {
    match provenance {
        OwnershipProvenance::UserDeclared => ProvenanceProjection {
            source: "user_declared".to_owned(),
            context_id: None,
            candidate_id: None,
            authority_sha256: None,
            row_sha256: None,
        },
        OwnershipProvenance::AdoptedManifest {
            context_id,
            candidate_id,
            authority_sha256,
            row_sha256,
        } => ProvenanceProjection {
            source: "adopted_manifest".to_owned(),
            context_id: Some(context_id.clone()),
            candidate_id: Some(candidate_id.clone()),
            authority_sha256: Some(authority_sha256.clone()),
            row_sha256: Some(row_sha256.clone()),
        },
    }
}

pub(crate) fn expected_projection(expected: &ExpectedContent) -> ExpectedProjection {
    match expected {
        ExpectedContent::Absent => ExpectedProjection::Absent,
        ExpectedContent::ExactDigest(value) => ExpectedProjection::ExactDigest(value.clone()),
    }
}

pub(crate) fn mode(context: &LiveContext) -> FitMode {
    if context.candidate().head_commit.is_none() {
        FitMode::Fresh
    } else {
        FitMode::Retrofit
    }
}

pub(crate) const fn mode_name(mode: FitMode) -> &'static str {
    match mode {
        FitMode::Fresh => "fresh",
        FitMode::Retrofit => "retrofit",
    }
}

pub(crate) const fn class_name(class: &RepositoryClass) -> &'static str {
    match class {
        RepositoryClass::Fresh => "fresh",
        RepositoryClass::Partial => "partial",
        RepositoryClass::Conflicting => "conflicting",
        RepositoryClass::AlreadyFitted => "already_fitted",
    }
}

pub(crate) const fn disposition_name(disposition: &ObservedDisposition) -> &'static str {
    match disposition {
        ObservedDisposition::Missing => "missing",
        ObservedDisposition::Matching => "matching",
        ObservedDisposition::ManagedOutdated => "managed_outdated",
        ObservedDisposition::Conflict => "conflict",
    }
}

pub(crate) const fn ownership_name(ownership: Ownership) -> &'static str {
    match ownership {
        Ownership::HarnessGenerated => "harness_generated",
        Ownership::UserOwned => "user_owned",
    }
}
