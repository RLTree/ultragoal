use super::*;

pub(crate) fn classify(files: &[ObservedFile]) -> RepositoryClass {
    if files
        .iter()
        .all(|file| file.disposition == ObservedDisposition::Missing)
    {
        RepositoryClass::Fresh
    } else if files
        .iter()
        .all(|file| file.disposition == ObservedDisposition::Matching)
    {
        RepositoryClass::AlreadyFitted
    } else if files
        .iter()
        .any(|file| file.disposition == ObservedDisposition::Conflict)
    {
        RepositoryClass::Conflicting
    } else {
        RepositoryClass::Partial
    }
}

pub(crate) fn inspection_digest(
    context: &str,
    candidate: &str,
    root: &str,
    desired: &str,
    mode: FitMode,
    class: &RepositoryClass,
    files: &[ObservedFile],
) -> Result<String, FitError> {
    #[derive(Serialize)]
    struct Row<'a> {
        path: &'a CanonicalPath,
        ownership: Ownership,
        provenance: &'a OwnershipProvenance,
        prior_proof_sha256: &'a Option<String>,
        disposition: &'a ObservedDisposition,
        observed_sha256: &'a Option<String>,
    }
    let rows = files
        .iter()
        .map(|file| Row {
            path: &file.path,
            ownership: file.ownership,
            provenance: &file.provenance,
            prior_proof_sha256: &file.prior_proof_sha256,
            disposition: &file.disposition,
            observed_sha256: &file.observed_sha256,
        })
        .collect::<Vec<_>>();
    serde_json::to_vec(&(context, candidate, root, desired, mode, class, rows))
        .map(|bytes| digest(&bytes))
        .map_err(|_| error(FitErrorId::InvalidSpec))
}

pub(crate) fn plan_digest(
    inspection: &FitInspection,
    desired: &DesiredState,
    checks: &[FitCheck],
    mutations: &[Mutation],
    conflicts: &[FitConflict],
    local_state: Option<&LocalStatePlan>,
) -> Result<String, FitError> {
    #[derive(Serialize)]
    struct Row<'a> {
        path: &'a CanonicalPath,
        expected: &'a ExpectedContent,
        replacement_sha256: String,
        replacement_bytes: usize,
        ownership: Ownership,
        provenance: &'a OwnershipProvenance,
        prior_proof_sha256: &'a Option<String>,
    }
    let rows = mutations
        .iter()
        .map(|mutation| Row {
            path: &mutation.path,
            expected: &mutation.expected,
            replacement_sha256: mutation.replacement_sha256(),
            replacement_bytes: mutation.replacement.len(),
            ownership: mutation.ownership,
            provenance: &mutation.provenance,
            prior_proof_sha256: &mutation.prior_proof_sha256,
        })
        .collect::<Vec<_>>();
    serde_json::to_vec(&(
        &inspection.context_id,
        &inspection.candidate_id,
        &inspection.root_binding,
        &desired.state_sha256,
        &inspection.inspection_sha256,
        checks,
        rows,
        conflicts,
        local_state.map(|state| {
            (
                state.path.as_str(),
                &state.expected,
                state.desired_sha256(),
                state.observed_mode,
                state.desired_mode,
                state.mutation.is_some(),
            )
        }),
    ))
    .map(|bytes| digest(&bytes))
    .map_err(|_| error(FitErrorId::InvalidSpec))
}
