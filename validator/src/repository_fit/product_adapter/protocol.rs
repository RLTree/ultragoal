use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

use crate::context::LiveContext;

use super::catalog::{DesiredBundle, compile};
use super::model::{
    APPLY_PREPARATION_SCHEMA, CLAIM_EFFECT, CandidateProjection, CheckProjection,
    ConflictProjection, DesiredProjection, ExpectedProjection, FitApplyPreparationProjection,
    FitInspectProjection, FitPlanRecord, FitVerificationProjection, INSPECT_SCHEMA,
    InspectionProjection, MutationProjection, ObservedFileProjection, PLAN_SCHEMA, PlanProjection,
    ProvenanceProjection, RollbackEntryProjection, SUPPORT_LIMIT, TargetProjection, VERIFY_SCHEMA,
    VerificationFailureProjection,
};
use super::{AdapterErrorId, FitAdapterError, adapter_error, kernel_error};
use crate::repository_fit::LocalEffects;
use crate::repository_fit::{
    DesiredState, ExpectedContent, FitInspection, FitMode, FitPlan, LocalRepository,
    ObservedDisposition, Ownership, OwnershipProvenance, PlanAuthorization, RepositoryClass,
    digest, inspect, plan, valid_digest, verify,
};

const MAX_PLAN_RECORD_BYTES: usize = 16 * 1024 * 1024;

struct CurrentPlan {
    target: TargetProjection,
    bundle: DesiredBundle,
    inspection: FitInspection,
    observed_modes: BTreeMap<String, Option<u32>>,
    plan: FitPlan,
}

/// A one-use, non-cloneable accepted-plan carrier. No method on this type
/// performs an effect; a separate root-owned permit boundary must consume it.
pub(crate) struct OpaqueFitApplyRequest {
    request_id: String,
    context_id: String,
    candidate_id: String,
    root_binding: String,
    desired: DesiredState,
    plan: FitPlan,
    authorization: PlanAuthorization,
    unix_modes: BTreeMap<String, u32>,
}

impl OpaqueFitApplyRequest {
    pub(crate) fn request_id(&self) -> &str {
        &self.request_id
    }

    pub(crate) fn context_id(&self) -> &str {
        &self.context_id
    }

    pub(crate) fn candidate_id(&self) -> &str {
        &self.candidate_id
    }

    pub(crate) fn root_binding(&self) -> &str {
        &self.root_binding
    }

    pub(crate) fn plan_sha256(&self) -> &str {
        self.plan.plan_sha256()
    }

    pub(crate) fn unix_modes(&self) -> &BTreeMap<String, u32> {
        &self.unix_modes
    }

    #[cfg(test)]
    pub(super) fn execute_for_test(
        self,
        effects: &mut impl crate::repository_fit::FitEffects,
    ) -> Result<crate::repository_fit::FitVerification, crate::repository_fit::FitError> {
        crate::repository_fit::apply(&self.plan, &self.authorization, effects)?;
        crate::repository_fit::verify(&self.desired, effects)
    }
}

pub(crate) struct PreparedFitApply {
    request: OpaqueFitApplyRequest,
    projection: FitApplyPreparationProjection,
}

impl PreparedFitApply {
    pub(crate) fn request(&self) -> &OpaqueFitApplyRequest {
        &self.request
    }

    pub(crate) fn projection(&self) -> &FitApplyPreparationProjection {
        &self.projection
    }

    pub(crate) fn into_request(self) -> OpaqueFitApplyRequest {
        self.request
    }
}

pub(crate) fn inspect_target(
    context: &LiveContext,
) -> Result<FitInspectProjection, FitAdapterError> {
    context
        .revalidate()
        .map_err(|_| adapter_error(AdapterErrorId::ContextStale))?;
    let target = target_projection(context)?;
    let bundle = compile(context)?;
    let mut reader = LocalRepository::open(context.worktree_root()).map_err(kernel_error)?;
    let mut inspection =
        inspect(mode(context), &bundle.desired, &mut reader).map_err(kernel_error)?;
    let observed_modes = bind_authoritative_modes(context, &bundle, &mut inspection)?;
    context
        .revalidate()
        .map_err(|_| adapter_error(AdapterErrorId::ContextStale))?;
    Ok(FitInspectProjection {
        schema_version: INSPECT_SCHEMA.to_owned(),
        target,
        authority: bundle.authority,
        desired: desired_projection(&bundle.desired),
        inspection: inspection_projection(&inspection, &observed_modes, &bundle.unix_modes),
        effect: "read".to_owned(),
        claim_effect: CLAIM_EFFECT.to_owned(),
        support_limit: SUPPORT_LIMIT.to_owned(),
    })
}

pub(crate) fn plan_target(context: &LiveContext) -> Result<FitPlanRecord, FitAdapterError> {
    let current = current_plan(context)?;
    plan_record(&current)
}

pub(crate) fn verify_target(
    context: &LiveContext,
) -> Result<FitVerificationProjection, FitAdapterError> {
    context
        .revalidate()
        .map_err(|_| adapter_error(AdapterErrorId::ContextStale))?;
    let target = target_projection(context)?;
    let bundle = compile(context)?;
    let mut reader = LocalRepository::open(context.worktree_root()).map_err(kernel_error)?;
    let mut inspection =
        inspect(mode(context), &bundle.desired, &mut reader).map_err(kernel_error)?;
    let observed_modes = bind_authoritative_modes(context, &bundle, &mut inspection)?;
    let inspection_projection =
        inspection_projection(&inspection, &observed_modes, &bundle.unix_modes);
    let desired = desired_projection(&bundle.desired);
    let projection = if inspection.classification == RepositoryClass::AlreadyFitted {
        let verification = verify(&bundle.desired, &mut reader).map_err(kernel_error)?;
        if verification.root_binding != inspection.root_binding {
            return Err(adapter_error(AdapterErrorId::ContextStale));
        }
        let byte_verification_sha256 = verification.verification_sha256;
        let verification_sha256 = digest(
            &serde_json::to_vec(&(
                "repository-fit-mode-bound-verification-v1",
                &byte_verification_sha256,
                &inspection.inspection_sha256,
                &bundle.authority.authority_sha256,
                &bundle.desired.state_sha256,
                &verification.root_binding,
            ))
            .map_err(|_| adapter_error(AdapterErrorId::ProjectionFailed))?,
        );
        FitVerificationProjection {
            schema_version: VERIFY_SCHEMA.to_owned(),
            target,
            authority: bundle.authority,
            desired,
            root_binding: verification.root_binding,
            inspection_sha256: inspection.inspection_sha256,
            matched_files: verification.matched_files,
            idempotent: verification.idempotent,
            byte_verification_sha256: Some(byte_verification_sha256),
            verification_sha256: Some(verification_sha256),
            failure: None,
            effect: "read".to_owned(),
            claim_effect: CLAIM_EFFECT.to_owned(),
            support_limit: SUPPORT_LIMIT.to_owned(),
        }
    } else {
        let causal_files = inspection_projection
            .files
            .iter()
            .filter(|row| row.disposition != "matching")
            .cloned()
            .collect::<Vec<_>>();
        FitVerificationProjection {
            schema_version: VERIFY_SCHEMA.to_owned(),
            target,
            authority: bundle.authority,
            desired,
            root_binding: inspection.root_binding,
            inspection_sha256: inspection.inspection_sha256,
            matched_files: inspection
                .files
                .iter()
                .filter(|row| row.disposition == ObservedDisposition::Matching)
                .count(),
            idempotent: false,
            byte_verification_sha256: None,
            verification_sha256: None,
            failure: Some(VerificationFailureProjection {
                error_id: "HUFIT-011".to_owned(),
                classification: inspection_projection.classification,
                compatibility: inspection_projection.compatibility,
                causal_files,
            }),
            effect: "read".to_owned(),
            claim_effect: CLAIM_EFFECT.to_owned(),
            support_limit: SUPPORT_LIMIT.to_owned(),
        }
    };
    context
        .revalidate()
        .map_err(|_| adapter_error(AdapterErrorId::ContextStale))?;
    Ok(projection)
}

pub(crate) fn prepare_apply_request(
    context: &LiveContext,
    plan_record_bytes: &[u8],
    accepted_plan_sha256: &str,
) -> Result<PreparedFitApply, FitAdapterError> {
    if plan_record_bytes.is_empty()
        || plan_record_bytes.len() > MAX_PLAN_RECORD_BYTES
        || !valid_digest(accepted_plan_sha256)
    {
        return Err(adapter_error(AdapterErrorId::InvalidPlanRecord));
    }
    let supplied: FitPlanRecord = serde_json::from_slice(plan_record_bytes)
        .map_err(|_| adapter_error(AdapterErrorId::InvalidPlanRecord))?;
    validate_record_constants(&supplied)?;
    let supplied_canonical = supplied.to_machine_bytes()?;
    if supplied_canonical != plan_record_bytes {
        return Err(adapter_error(AdapterErrorId::InvalidPlanRecord));
    }
    if supplied.plan.plan_sha256 != accepted_plan_sha256 {
        return Err(adapter_error(AdapterErrorId::AcceptanceMismatch));
    }

    let current = current_plan(context)?;
    let recomputed = plan_record(&current)?;
    if recomputed.to_machine_bytes()? != supplied_canonical {
        return Err(adapter_error(AdapterErrorId::StalePlan));
    }
    if !current.plan.conflicts.is_empty() {
        return Err(adapter_error(AdapterErrorId::PlanConflict));
    }
    context
        .revalidate()
        .map_err(|_| adapter_error(AdapterErrorId::ContextStale))?;
    let authorization = PlanAuthorization::new(
        current.target.context_id.clone(),
        current.target.candidate.candidate_id.clone(),
        current.plan.plan_sha256.clone(),
    )
    .map_err(kernel_error)?;
    let request_id = digest(
        &serde_json::to_vec(&(
            "repository-fit-opaque-apply-request-v1",
            &current.target.context_id,
            &current.target.candidate.candidate_id,
            &current.inspection.root_binding,
            &current.bundle.desired.state_sha256,
            &current.plan.plan_sha256,
            accepted_plan_sha256,
            &current.bundle.authority.authority_sha256,
        ))
        .map_err(|_| adapter_error(AdapterErrorId::ProjectionFailed))?,
    );
    let projection = FitApplyPreparationProjection {
        schema_version: APPLY_PREPARATION_SCHEMA.to_owned(),
        request_id: request_id.clone(),
        context_id: current.target.context_id.clone(),
        candidate_id: current.target.candidate.candidate_id.clone(),
        root_binding: current.inspection.root_binding.clone(),
        desired_state_sha256: current.bundle.desired.state_sha256.clone(),
        plan_sha256: current.plan.plan_sha256.clone(),
        accepted_plan_sha256: accepted_plan_sha256.to_owned(),
        mutation_count: current.plan.mutations.len(),
        effect: "workspace_write_not_executed".to_owned(),
        claim_effect: CLAIM_EFFECT.to_owned(),
        support_limit: SUPPORT_LIMIT.to_owned(),
    };
    let request = OpaqueFitApplyRequest {
        request_id,
        context_id: current.target.context_id,
        candidate_id: current.target.candidate.candidate_id,
        root_binding: current.inspection.root_binding,
        desired: current.bundle.desired,
        plan: current.plan,
        authorization,
        unix_modes: current.bundle.unix_modes,
    };
    Ok(PreparedFitApply {
        request,
        projection,
    })
}

fn current_plan(context: &LiveContext) -> Result<CurrentPlan, FitAdapterError> {
    context
        .revalidate()
        .map_err(|_| adapter_error(AdapterErrorId::ContextStale))?;
    let target = target_projection(context)?;
    let bundle = compile(context)?;
    let mut reader = LocalRepository::open(context.worktree_root()).map_err(kernel_error)?;
    let mut inspection =
        inspect(mode(context), &bundle.desired, &mut reader).map_err(kernel_error)?;
    let observed_modes = bind_authoritative_modes(context, &bundle, &mut inspection)?;
    let plan = plan(&inspection, &bundle.desired).map_err(kernel_error)?;
    context
        .revalidate()
        .map_err(|_| adapter_error(AdapterErrorId::ContextStale))?;
    Ok(CurrentPlan {
        target,
        bundle,
        inspection,
        observed_modes,
        plan,
    })
}

fn plan_record(current: &CurrentPlan) -> Result<FitPlanRecord, FitAdapterError> {
    Ok(FitPlanRecord {
        schema_version: PLAN_SCHEMA.to_owned(),
        target: current.target.clone(),
        authority: current.bundle.authority.clone(),
        desired: desired_projection(&current.bundle.desired),
        inspection: inspection_projection(
            &current.inspection,
            &current.observed_modes,
            &current.bundle.unix_modes,
        ),
        plan: plan_projection(
            &current.plan,
            &current.observed_modes,
            &current.bundle.unix_modes,
        )?,
        effect: "read".to_owned(),
        claim_effect: CLAIM_EFFECT.to_owned(),
        support_limit: SUPPORT_LIMIT.to_owned(),
    })
}

fn validate_record_constants(record: &FitPlanRecord) -> Result<(), FitAdapterError> {
    if record.schema_version != PLAN_SCHEMA
        || record.effect != "read"
        || record.claim_effect != CLAIM_EFFECT
        || record.support_limit != SUPPORT_LIMIT
        || !valid_digest(&record.target.context_id)
        || !valid_digest(&record.target.candidate.candidate_id)
        || !valid_digest(&record.plan.plan_sha256)
    {
        return Err(adapter_error(AdapterErrorId::InvalidPlanRecord));
    }
    Ok(())
}

fn target_projection(context: &LiveContext) -> Result<TargetProjection, FitAdapterError> {
    let candidate_bytes = serde_json::to_vec(context.candidate())
        .map_err(|_| adapter_error(AdapterErrorId::ProjectionFailed))?;
    let candidate_id = digest(&candidate_bytes);
    Ok(TargetProjection {
        context_id: context.context_id().to_owned(),
        repository_root_id: root_id(
            context.context_id(),
            "repository",
            &context.roots().repository_root,
        ),
        worktree_root_id: root_id(
            context.context_id(),
            "worktree",
            &context.roots().worktree_root,
        ),
        candidate: CandidateProjection {
            candidate_id,
            head_commit: context.candidate().head_commit.clone(),
            head_tree: context.candidate().head_tree.clone(),
            branch: context.candidate().branch.clone(),
            status_sha256: context.candidate().status_sha256.clone(),
            worktree_diff_sha256: context.candidate().worktree_diff_sha256.clone(),
            staged_diff_sha256: context.candidate().staged_diff_sha256.clone(),
            untracked_content_sha256: context.candidate().untracked_content_sha256.clone(),
            dirty: context.candidate().dirty,
        },
    })
}

fn desired_projection(desired: &DesiredState) -> DesiredProjection {
    DesiredProjection {
        state_sha256: desired.state_sha256.clone(),
        file_count: desired.files.len(),
    }
}

fn bind_authoritative_modes(
    context: &LiveContext,
    bundle: &DesiredBundle,
    inspection: &mut FitInspection,
) -> Result<BTreeMap<String, Option<u32>>, FitAdapterError> {
    let mut reader = LocalEffects::open(context.worktree_root(), bundle.unix_modes.clone())
        .map_err(kernel_error)?;
    let mut observed_modes = BTreeMap::new();
    for row in &mut inspection.files {
        let desired_mode = bundle
            .unix_modes
            .get(row.path.as_str())
            .copied()
            .ok_or_else(|| adapter_error(AdapterErrorId::InvalidTemplateCatalog))?;
        let observed_mode = reader.read_unix_mode(&row.path).map_err(kernel_error)?;
        if row.observed_sha256.is_none() != observed_mode.is_none() {
            return Err(adapter_error(AdapterErrorId::TargetUnavailable));
        }
        if row.disposition == ObservedDisposition::Matching && observed_mode != Some(desired_mode) {
            row.disposition = ObservedDisposition::ManagedOutdated;
        }
        observed_modes.insert(row.path.as_str().to_owned(), observed_mode);
    }
    inspection.classification = if inspection
        .files
        .iter()
        .all(|row| row.disposition == ObservedDisposition::Missing)
    {
        RepositoryClass::Fresh
    } else if inspection
        .files
        .iter()
        .all(|row| row.disposition == ObservedDisposition::Matching)
    {
        RepositoryClass::AlreadyFitted
    } else if inspection
        .files
        .iter()
        .any(|row| row.disposition == ObservedDisposition::Conflict)
    {
        RepositoryClass::Conflicting
    } else {
        RepositoryClass::Partial
    };
    #[derive(Serialize)]
    struct ModeRow<'a> {
        path: &'a str,
        disposition: &'a ObservedDisposition,
        observed_sha256: &'a Option<String>,
        observed_unix_mode: Option<u32>,
        desired_unix_mode: u32,
    }
    let rows = inspection
        .files
        .iter()
        .map(|row| ModeRow {
            path: row.path.as_str(),
            disposition: &row.disposition,
            observed_sha256: &row.observed_sha256,
            observed_unix_mode: observed_modes[row.path.as_str()],
            desired_unix_mode: bundle.unix_modes[row.path.as_str()],
        })
        .collect::<Vec<_>>();
    inspection.inspection_sha256 = digest(
        &serde_json::to_vec(&(
            "repository-fit-mode-bound-inspection-v1",
            &inspection.context_id,
            &inspection.candidate_id,
            &inspection.root_binding,
            &inspection.desired_state_sha256,
            inspection.mode,
            &inspection.classification,
            rows,
        ))
        .map_err(|_| adapter_error(AdapterErrorId::ProjectionFailed))?,
    );
    Ok(observed_modes)
}

fn inspection_projection(
    inspection: &FitInspection,
    observed_modes: &BTreeMap<String, Option<u32>>,
    desired_modes: &BTreeMap<String, u32>,
) -> InspectionProjection {
    InspectionProjection {
        mode: mode_name(inspection.mode).to_owned(),
        classification: class_name(&inspection.classification).to_owned(),
        compatibility: if inspection.classification == RepositoryClass::Conflicting {
            "conflicting"
        } else {
            "compatible"
        }
        .to_owned(),
        root_binding: inspection.root_binding.clone(),
        inspection_sha256: inspection.inspection_sha256.clone(),
        files: inspection
            .files
            .iter()
            .map(|row| observed_projection(row, observed_modes, desired_modes))
            .collect(),
    }
}

fn observed_projection(
    row: &crate::repository_fit::state::ObservedFile,
    observed_modes: &BTreeMap<String, Option<u32>>,
    desired_modes: &BTreeMap<String, u32>,
) -> ObservedFileProjection {
    ObservedFileProjection {
        path: row.path.as_str().to_owned(),
        ownership: ownership_name(row.ownership).to_owned(),
        provenance: provenance_projection(&row.provenance),
        disposition: disposition_name(&row.disposition).to_owned(),
        observed_sha256: row.observed_sha256.clone(),
        observed_unix_mode: observed_modes.get(row.path.as_str()).copied().flatten(),
        desired_unix_mode: desired_modes[row.path.as_str()],
        prior_proof_sha256: row.prior_proof_sha256.clone(),
    }
}

fn plan_projection(
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

fn conflict_projection(
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

fn provenance_projection(provenance: &OwnershipProvenance) -> ProvenanceProjection {
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

fn expected_projection(expected: &ExpectedContent) -> ExpectedProjection {
    match expected {
        ExpectedContent::Absent => ExpectedProjection::Absent,
        ExpectedContent::ExactDigest(value) => ExpectedProjection::ExactDigest(value.clone()),
    }
}

fn mode(context: &LiveContext) -> FitMode {
    if context.candidate().head_commit.is_none() {
        FitMode::Fresh
    } else {
        FitMode::Retrofit
    }
}

const fn mode_name(mode: FitMode) -> &'static str {
    match mode {
        FitMode::Fresh => "fresh",
        FitMode::Retrofit => "retrofit",
    }
}

const fn class_name(class: &RepositoryClass) -> &'static str {
    match class {
        RepositoryClass::Fresh => "fresh",
        RepositoryClass::Partial => "partial",
        RepositoryClass::Conflicting => "conflicting",
        RepositoryClass::AlreadyFitted => "already_fitted",
    }
}

const fn disposition_name(disposition: &ObservedDisposition) -> &'static str {
    match disposition {
        ObservedDisposition::Missing => "missing",
        ObservedDisposition::Matching => "matching",
        ObservedDisposition::ManagedOutdated => "managed_outdated",
        ObservedDisposition::Conflict => "conflict",
    }
}

const fn ownership_name(ownership: Ownership) -> &'static str {
    match ownership {
        Ownership::HarnessGenerated => "harness_generated",
        Ownership::UserOwned => "user_owned",
    }
}

fn root_id(context_id: &str, role: &str, absolute: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"repository-fit-root-id-v1\0");
    hasher.update(context_id.as_bytes());
    hasher.update([0]);
    hasher.update(role.as_bytes());
    hasher.update([0]);
    hasher.update(absolute.as_bytes());
    format!("sha256:{:x}", hasher.finalize())
}

#[allow(dead_code)]
fn _assert_serializable<T: Serialize>(_value: &T) {}
