use super::*;

pub(crate) const MAX_PLAN_RECORD_BYTES: u64 = 16 * 1024 * 1024;

pub(crate) fn target_root(
    root: &Path,
    invocation: &ParsedInvocation,
) -> Result<PathBuf, RuntimeOutcome> {
    let mut target = None;
    for argument in &invocation.arguments {
        match (&argument.name, &argument.value) {
            (OptionName::Target, ParsedValue::RelativePath(path)) if target.is_none() => {
                target = Some(path.as_str())
            }
            (OptionName::Plan, ParsedValue::RelativePath(_))
            | (OptionName::AcceptPlan, ParsedValue::Identifier(_))
                if invocation.command == SuccessorCommand::Fit(FitAction::Apply) => {}
            _ => return Err(invalid_invocation()),
        }
    }
    Ok(target.map_or_else(|| root.to_path_buf(), |path| root.join(path)))
}

pub(crate) fn inspect(context: &LiveContext, invocation: &ParsedInvocation) -> RuntimeOutcome {
    if !valid_read_invocation(invocation, FitAction::Inspect) {
        return invalid_invocation();
    }
    match inspect_target(context) {
        Ok(projection) => match projection.to_machine_bytes() {
            Ok(machine) => {
                let finding = !projection.compatible();
                RuntimeOutcome::payload(
                    if finding {
                        ExitClass::ActionableFinding
                    } else {
                        ExitClass::Success
                    },
                    machine,
                    format!(
                        "repository fit {} files={} claim_effect=none",
                        projection.classification(),
                        projection.file_count()
                    ),
                )
            }
            Err(failure) => adapter_failure(failure),
        },
        Err(failure) => adapter_failure(failure),
    }
}

pub(crate) fn plan(context: &LiveContext, invocation: &ParsedInvocation) -> RuntimeOutcome {
    if !valid_read_invocation(invocation, FitAction::Plan) {
        return invalid_invocation();
    }
    match plan_target(context) {
        Ok(record) => match record.to_machine_bytes() {
            Ok(machine) => RuntimeOutcome::payload(
                if record.conflict_count() == 0 {
                    ExitClass::Success
                } else {
                    ExitClass::ActionableFinding
                },
                machine,
                format!(
                    "repository fit plan {} mutations={} conflicts={} claim_effect=none",
                    record.plan_sha256(),
                    record.mutation_count(),
                    record.conflict_count()
                ),
            ),
            Err(failure) => adapter_failure(failure),
        },
        Err(failure) => adapter_failure(failure),
    }
}

pub(crate) fn verify(context: &LiveContext, invocation: &ParsedInvocation) -> RuntimeOutcome {
    if !valid_read_invocation(invocation, FitAction::Verify) {
        return invalid_invocation();
    }
    match verify_target(context) {
        Ok(projection) => match projection.to_machine_bytes() {
            Ok(machine) => RuntimeOutcome::payload(
                if projection.idempotent() {
                    ExitClass::Success
                } else {
                    ExitClass::ActionableFinding
                },
                machine,
                format!(
                    "repository fit verified={} matched_files={} claim_effect=none",
                    projection.idempotent(),
                    projection.matched_files()
                ),
            ),
            Err(failure) => adapter_failure(failure),
        },
        Err(failure) => adapter_failure(failure),
    }
}

pub(crate) fn apply(
    context: &LiveContext,
    invocation: &ParsedInvocation,
    home: Option<&Path>,
) -> RuntimeOutcome {
    let arguments = match apply_arguments(invocation) {
        Ok(arguments) => arguments,
        Err(outcome) => return outcome,
    };
    let Some(home) = home else {
        return authority::host_state_unavailable();
    };
    match authority::recover_pending(context, home) {
        Ok(Some(outcome)) => return outcome,
        Ok(None) => {}
        Err(outcome) => return outcome,
    }
    let prepared = match prepare_apply_with_arguments(context, arguments) {
        Ok(prepared) => prepared,
        Err(outcome) => return outcome,
    };
    authority::execute(context, prepared, home)
}

pub(crate) struct ApplyArguments<'a> {
    pub(crate) plan_path: &'a str,
    pub(crate) accepted_plan: &'a str,
}

pub(crate) fn apply_arguments(
    invocation: &ParsedInvocation,
) -> Result<ApplyArguments<'_>, RuntimeOutcome> {
    if invocation.command != SuccessorCommand::Fit(FitAction::Apply)
        || invocation.effect != EffectClass::WorkspaceWrite
    {
        return Err(invalid_invocation());
    }
    let mut plan_path = None;
    let mut accepted_plan = None;
    for argument in &invocation.arguments {
        match (&argument.name, &argument.value) {
            (OptionName::Target, ParsedValue::RelativePath(_)) => {}
            (OptionName::Plan, ParsedValue::RelativePath(path)) if plan_path.is_none() => {
                plan_path = Some(path.as_str())
            }
            (OptionName::AcceptPlan, ParsedValue::Identifier(value)) if accepted_plan.is_none() => {
                accepted_plan = Some(value.as_str())
            }
            _ => return Err(invalid_invocation()),
        }
    }
    let (Some(plan_path), Some(accepted_plan)) = (plan_path, accepted_plan) else {
        return Err(invalid_invocation());
    };
    Ok(ApplyArguments {
        plan_path,
        accepted_plan,
    })
}

/// Reads one descriptor-anchored bounded plan record and returns an opaque
/// request. The shell-facing JSON renderer adds exactly one LF; accepting only
/// that framing in addition to canonical bytes keeps ordinary redirection
/// usable without accepting general whitespace or alternate encodings.
pub(crate) fn prepare_apply_with_arguments(
    context: &LiveContext,
    arguments: ApplyArguments<'_>,
) -> Result<PreparedFitApply, RuntimeOutcome> {
    let reads = context.begin_read_session().map_err(|_| stale_context())?;
    let absolute = reads.root().join(arguments.plan_path);
    let bytes = reads
        .read_bounded(&absolute, MAX_PLAN_RECORD_BYTES)
        .map_err(|_| invalid_plan())?;
    reads.revalidate().map_err(|_| stale_context())?;
    let canonical = if bytes.ends_with(b"\n") && !bytes[..bytes.len() - 1].ends_with(b"\n") {
        &bytes[..bytes.len() - 1]
    } else {
        bytes.as_slice()
    };
    prepare_apply_request(context, canonical, arguments.accepted_plan).map_err(adapter_failure)
}

pub(crate) fn valid_read_invocation(invocation: &ParsedInvocation, action: FitAction) -> bool {
    invocation.command == SuccessorCommand::Fit(action)
        && invocation.effect == EffectClass::Read
        && invocation.arguments.iter().all(|argument| {
            matches!(
                (&argument.name, &argument.value),
                (OptionName::Target, ParsedValue::RelativePath(_))
            )
        })
        && invocation
            .arguments
            .iter()
            .filter(|argument| argument.name == OptionName::Target)
            .count()
            <= 1
}
