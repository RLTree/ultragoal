use super::*;

#[cfg(target_vendor = "apple")]
pub(crate) fn recovery_target_spec(
    request: &super::super::OpaqueFitApplyRequest,
    ancestors: ManagedAncestorContract,
) -> Result<RecoveryTargetSpec, FitAdapterError> {
    let mut rows = Vec::with_capacity(request.desired.files.len());
    for desired in &request.desired.files {
        let path = desired.path.as_str();
        let post_mode = request
            .unix_modes
            .get(path)
            .copied()
            .ok_or_else(|| adapter_error(AdapterErrorId::ApplyPermitInvalid))?;
        let mutation = request
            .plan
            .mutations
            .iter()
            .find(|mutation| mutation.path == desired.path);
        let (pre_sha256, pre_mode) = match mutation {
            Some(mutation) => match &mutation.expected {
                ExpectedContent::Absent => {
                    if mutation.prior.is_some()
                        || request
                            .observed_modes
                            .get(path)
                            .copied()
                            .flatten()
                            .is_some()
                    {
                        return Err(adapter_error(AdapterErrorId::ApplyPermitInvalid));
                    }
                    (None, None)
                }
                ExpectedContent::ExactDigest(expected) => {
                    let prior = mutation
                        .prior
                        .as_ref()
                        .ok_or_else(|| adapter_error(AdapterErrorId::ApplyPermitInvalid))?;
                    if digest(prior) != *expected {
                        return Err(adapter_error(AdapterErrorId::ApplyPermitInvalid));
                    }
                    let mode = request
                        .observed_modes
                        .get(path)
                        .copied()
                        .flatten()
                        .ok_or_else(|| adapter_error(AdapterErrorId::ApplyPermitInvalid))?;
                    (Some(expected.clone()), Some(mode))
                }
            },
            None => {
                let mode = request
                    .observed_modes
                    .get(path)
                    .copied()
                    .flatten()
                    .ok_or_else(|| adapter_error(AdapterErrorId::ApplyPermitInvalid))?;
                (Some(desired.sha256()), Some(mode))
            }
        };
        rows.push(RecoveryTargetRow {
            path: path.to_owned(),
            pre_sha256,
            pre_mode,
            post_sha256: desired.sha256(),
            post_mode,
        });
    }
    rows.sort_by(|left, right| left.path.as_bytes().cmp(right.path.as_bytes()));
    Ok(RecoveryTargetSpec {
        request_id: request.request_id().to_owned(),
        root_binding: request.root_binding().to_owned(),
        ancestors,
        rows,
    })
}

pub(crate) fn valid_recovery_target_spec(recovery: &RecoveryTargetSpec) -> bool {
    let leaf_paths = recovery
        .rows
        .iter()
        .map(|row| row.path.clone())
        .collect::<Vec<_>>();
    valid_digest(&recovery.request_id)
        && valid_digest(&recovery.root_binding)
        && !recovery.rows.is_empty()
        && recovery.rows.len() <= MAX_RECOVERY_ROWS
        && recovery.ancestors.valid_for_leaf_paths(&leaf_paths)
        && recovery.rows.iter().all(|row| {
            CanonicalPath::parse(&row.path).is_ok()
                && row.pre_sha256.as_deref().is_none_or(valid_digest)
                && row
                    .pre_mode
                    .is_none_or(|mode| matches!(mode, 0o644 | 0o755))
                && row.pre_sha256.is_some() == row.pre_mode.is_some()
                && valid_digest(&row.post_sha256)
                && matches!(row.post_mode, 0o644 | 0o755)
        })
        && recovery
            .rows
            .windows(2)
            .all(|rows| rows[0].path.as_bytes() < rows[1].path.as_bytes())
}

pub(crate) fn refuse_existing_execution(
    existing: super::super::ledger::ExistingReservation,
    current_intent: &RepositoryFitRecoveryIntent,
    request_id: String,
) -> RepositoryFitProductionOutcome {
    let observed_state = existing.state();
    let observed_effect_started = observed_state == RepositoryFitLedgerState::EffectStarted;
    let error = if !existing.matches_intent(current_intent.sha256(), current_intent.recovery())
        || observed_state.terminal()
    {
        AdapterErrorId::ApplyPermitReplayed
    } else {
        AdapterErrorId::ApplyLeaseInvalid
    };
    RepositoryFitProductionOutcome::causal_refusal(
        request_id,
        adapter_error(error),
        observed_state,
        observed_effect_started,
    )
}
