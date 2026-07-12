use super::state::ObservedFile;
use super::{
    CanonicalPath, DesiredState, ExpectedContent, FitCheck, FitConflict, FitError, FitErrorId,
    FitInspection, FitMode, FitPlan, FitReader, ManagedPriorProof, Mutation, ObservedDisposition,
    Ownership, OwnershipProvenance, RepositoryClass, RollbackPlan, digest, error, valid_digest,
};
use serde::Serialize;

pub fn inspect(
    mode: FitMode,
    desired: &DesiredState,
    reader: &mut impl FitReader,
) -> Result<FitInspection, FitError> {
    inspect_with_managed_proofs(mode, desired, &[], reader)
}

pub fn inspect_with_managed_proofs(
    mode: FitMode,
    desired: &DesiredState,
    proofs: &[ManagedPriorProof],
    reader: &mut impl FitReader,
) -> Result<FitInspection, FitError> {
    if proofs.len() > desired.files.len() {
        return Err(error(FitErrorId::InvalidSpec));
    }
    let root_binding = reader.root_binding()?;
    if !valid_digest(&root_binding) {
        return Err(error(FitErrorId::StaleBinding));
    }
    let mut files = Vec::with_capacity(desired.files.len());
    let mut used_proofs = vec![false; proofs.len()];
    for wanted in &desired.files {
        let prior = reader.read_file(&wanted.path, super::model::MAX_FILE_BYTES)?;
        if prior
            .as_ref()
            .is_some_and(|bytes| bytes.len() > super::model::MAX_FILE_BYTES)
        {
            return Err(error(FitErrorId::ResourceLimit));
        }
        let observed_sha256 = prior.as_deref().map(digest);
        let mut prior_proof_sha256 = None;
        let disposition = match observed_sha256.as_deref() {
            None => ObservedDisposition::Missing,
            Some(value) if value == wanted.sha256() => ObservedDisposition::Matching,
            Some(value)
                if wanted.ownership == Ownership::HarnessGenerated
                    && wanted.known_prior.contains(value) =>
            {
                let matches = proofs
                    .iter()
                    .enumerate()
                    .filter(|(_, proof)| {
                        proof.matches(
                            &desired.context_id,
                            &desired.candidate_id,
                            &root_binding,
                            &wanted.path,
                            value,
                            &wanted.provenance,
                        )
                    })
                    .map(|(index, _)| index)
                    .collect::<Vec<_>>();
                if matches.len() > 1 {
                    return Err(error(FitErrorId::InvalidSpec));
                }
                if let Some(index) = matches.first().copied() {
                    used_proofs[index] = true;
                    prior_proof_sha256 = Some(proofs[index].proof_sha256().to_owned());
                    ObservedDisposition::ManagedOutdated
                } else {
                    ObservedDisposition::Conflict
                }
            }
            Some(_) => ObservedDisposition::Conflict,
        };
        files.push(ObservedFile {
            path: wanted.path.clone(),
            ownership: wanted.ownership,
            provenance: wanted.provenance.clone(),
            prior_proof_sha256,
            disposition,
            observed_sha256,
            prior,
        });
    }
    if used_proofs.iter().any(|used| !used) {
        return Err(error(FitErrorId::InvalidSpec));
    }
    if reader.root_binding()? != root_binding {
        return Err(error(FitErrorId::StaleBinding));
    }
    let classification = classify(&files);
    let inspection_sha256 = inspection_digest(
        &desired.context_id,
        &desired.candidate_id,
        &root_binding,
        &desired.state_sha256,
        mode,
        &classification,
        &files,
    )?;
    Ok(FitInspection {
        context_id: desired.context_id.clone(),
        candidate_id: desired.candidate_id.clone(),
        root_binding,
        desired_state_sha256: desired.state_sha256.clone(),
        mode,
        classification,
        files,
        inspection_sha256,
    })
}

pub fn plan(inspection: &FitInspection, desired: &DesiredState) -> Result<FitPlan, FitError> {
    if inspection.context_id != desired.context_id
        || inspection.candidate_id != desired.candidate_id
        || inspection.desired_state_sha256 != desired.state_sha256
        || inspection.files.len() != desired.files.len()
    {
        return Err(error(FitErrorId::StaleBinding));
    }
    let mut mutations = Vec::new();
    let mut conflicts = Vec::new();
    let mut checks = Vec::new();
    for (observed, wanted) in inspection.files.iter().zip(&desired.files) {
        if observed.path != wanted.path
            || observed.ownership != wanted.ownership
            || observed.provenance != wanted.provenance
        {
            return Err(error(FitErrorId::StaleBinding));
        }
        let expected = observed
            .observed_sha256
            .clone()
            .map(ExpectedContent::ExactDigest)
            .unwrap_or(ExpectedContent::Absent);
        checks.push(FitCheck {
            path: wanted.path.clone(),
            expected: expected.clone(),
            desired_sha256: wanted.sha256(),
            provenance: wanted.provenance.clone(),
            prior_proof_sha256: observed.prior_proof_sha256.clone(),
        });
        match observed.disposition {
            ObservedDisposition::Missing => mutations.push(Mutation {
                path: wanted.path.clone(),
                expected,
                replacement: wanted.bytes.clone(),
                prior: None,
                ownership: wanted.ownership,
                provenance: wanted.provenance.clone(),
                prior_proof_sha256: observed.prior_proof_sha256.clone(),
            }),
            ObservedDisposition::ManagedOutdated => mutations.push(Mutation {
                path: wanted.path.clone(),
                expected,
                replacement: wanted.bytes.clone(),
                prior: observed.prior.clone(),
                ownership: wanted.ownership,
                provenance: wanted.provenance.clone(),
                prior_proof_sha256: observed.prior_proof_sha256.clone(),
            }),
            ObservedDisposition::Conflict => conflicts.push(FitConflict::new(
                wanted.path.clone(),
                observed
                    .observed_sha256
                    .clone()
                    .ok_or_else(|| error(FitErrorId::StaleBinding))?,
                wanted.ownership,
                wanted.provenance.clone(),
                observed.prior_proof_sha256.clone(),
            )),
            ObservedDisposition::Matching => {}
        }
    }
    let plan_sha256 = plan_digest(inspection, desired, &checks, &mutations, &conflicts)?;
    Ok(FitPlan {
        context_id: desired.context_id.clone(),
        candidate_id: desired.candidate_id.clone(),
        root_binding: inspection.root_binding.clone(),
        desired_state_sha256: desired.state_sha256.clone(),
        inspection_sha256: inspection.inspection_sha256.clone(),
        checks,
        rollback: RollbackPlan {
            mutation_count: mutations.len(),
        },
        mutations,
        conflicts,
        plan_sha256,
    })
}

fn classify(files: &[ObservedFile]) -> RepositoryClass {
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

fn inspection_digest(
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

fn plan_digest(
    inspection: &FitInspection,
    desired: &DesiredState,
    checks: &[FitCheck],
    mutations: &[Mutation],
    conflicts: &[FitConflict],
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
    ))
    .map(|bytes| digest(&bytes))
    .map_err(|_| error(FitErrorId::InvalidSpec))
}
