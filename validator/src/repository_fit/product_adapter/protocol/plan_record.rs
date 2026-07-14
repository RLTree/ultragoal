use super::*;

pub(crate) fn plan_record(current: &CurrentPlan) -> Result<FitPlanRecord, FitAdapterError> {
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

pub(crate) fn validate_record_constants(record: &FitPlanRecord) -> Result<(), FitAdapterError> {
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

pub(crate) fn target_projection(
    context: &LiveContext,
) -> Result<TargetProjection, FitAdapterError> {
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

pub(crate) fn desired_projection(desired: &DesiredState) -> DesiredProjection {
    DesiredProjection {
        state_sha256: desired.state_sha256.clone(),
        file_count: desired.files.len(),
    }
}

pub(crate) fn bind_authoritative_modes(
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

pub(crate) fn inspection_projection(
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

pub(crate) fn observed_projection(
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
