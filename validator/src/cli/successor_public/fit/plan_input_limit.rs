use super::*;

pub(crate) const MAX_PLAN_RECORD_BYTES: u64 = 16 * 1024 * 1024;

pub(crate) fn target_root(
    root: &Path,
    invocation: &ParsedInvocation,
) -> Result<PathBuf, Box<RuntimeOutcome>> {
    let mut target = None;
    for argument in &invocation.arguments {
        match (&argument.name, &argument.value) {
            (OptionName::Target, ParsedValue::RepositoryTarget(path)) if target.is_none() => {
                target = Some(path.as_str())
            }
            (OptionName::Plan, ParsedValue::HostPath(_))
            | (OptionName::AcceptPlan, ParsedValue::Identifier(_))
                if invocation.command == SuccessorCommand::Fit(FitAction::Apply) => {}
            (OptionName::RoutineConfig | OptionName::LocalState, ParsedValue::Flag)
                if invocation.command == SuccessorCommand::Fit(FitAction::Plan) => {}
            _ => return Err(Box::new(invalid_invocation())),
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
    let scope = match plan_scope(invocation) {
        Ok(scope) => scope,
        Err(outcome) => return *outcome,
    };
    let result = match scope {
        FitPlanScope::CompleteRepository => plan_target(context),
        FitPlanScope::RoutineConfiguration | FitPlanScope::LocalState => {
            plan_target_for_scope(context, scope)
        }
    };
    match result {
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

fn plan_scope(invocation: &ParsedInvocation) -> Result<FitPlanScope, Box<RuntimeOutcome>> {
    if invocation.command != SuccessorCommand::Fit(FitAction::Plan)
        || invocation.effect != EffectClass::Read
    {
        return Err(Box::new(invalid_invocation()));
    }
    let mut target_seen = false;
    let mut routine_configuration = false;
    let mut local_state = false;
    for argument in &invocation.arguments {
        match (&argument.name, &argument.value) {
            (OptionName::Target, ParsedValue::RepositoryTarget(_)) if !target_seen => {
                target_seen = true
            }
            (OptionName::RoutineConfig, ParsedValue::Flag) if !routine_configuration => {
                routine_configuration = true
            }
            (OptionName::LocalState, ParsedValue::Flag) if !local_state => local_state = true,
            _ => return Err(Box::new(invalid_invocation())),
        }
    }
    Ok(if routine_configuration && !local_state {
        FitPlanScope::RoutineConfiguration
    } else if local_state && !routine_configuration {
        FitPlanScope::LocalState
    } else if !routine_configuration && !local_state {
        FitPlanScope::CompleteRepository
    } else {
        return Err(Box::new(invalid_invocation()));
    })
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
        Err(outcome) => return *outcome,
    };
    let Some(home) = home else {
        return authority::host_state_unavailable();
    };
    match authority::recover_pending(context, home) {
        Ok(Some(outcome)) => return outcome,
        Ok(None) => {}
        Err(outcome) => return *outcome,
    }
    let prepared = match prepare_apply_with_arguments(context, arguments) {
        Ok(prepared) => prepared,
        Err(outcome) => return *outcome,
    };
    if context.revalidate().is_err() {
        return stale_context();
    }
    authority::execute(context, prepared, home)
}

pub(crate) struct ApplyArguments<'a> {
    pub(crate) plan_path: &'a Path,
    pub(crate) accepted_plan: &'a str,
}

pub(crate) fn apply_arguments(
    invocation: &ParsedInvocation,
) -> Result<ApplyArguments<'_>, Box<RuntimeOutcome>> {
    if invocation.command != SuccessorCommand::Fit(FitAction::Apply)
        || invocation.effect != EffectClass::WorkspaceWrite
    {
        return Err(Box::new(invalid_invocation()));
    }
    let mut plan_path = None;
    let mut accepted_plan = None;
    for argument in &invocation.arguments {
        match (&argument.name, &argument.value) {
            (OptionName::Target, ParsedValue::RepositoryTarget(_)) => {}
            (OptionName::Plan, ParsedValue::HostPath(path)) if plan_path.is_none() => {
                plan_path = Some(path.as_path())
            }
            (OptionName::AcceptPlan, ParsedValue::Identifier(value)) if accepted_plan.is_none() => {
                accepted_plan = Some(value.as_str())
            }
            _ => return Err(Box::new(invalid_invocation())),
        }
    }
    let (Some(plan_path), Some(accepted_plan)) = (plan_path, accepted_plan) else {
        return Err(Box::new(invalid_invocation()));
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
) -> Result<PreparedFitApply, Box<RuntimeOutcome>> {
    context
        .revalidate()
        .map_err(|_| Box::new(stale_context()))?;
    let bytes =
        super::external_plan_file::read_immutable_plan(arguments.plan_path, MAX_PLAN_RECORD_BYTES)
            .map_err(|_| Box::new(invalid_plan()))?;
    context
        .revalidate()
        .map_err(|_| Box::new(stale_context()))?;
    let canonical = if bytes.ends_with(b"\n") && !bytes[..bytes.len() - 1].ends_with(b"\n") {
        &bytes[..bytes.len() - 1]
    } else {
        bytes.as_slice()
    };
    prepare_apply_request(context, canonical, arguments.accepted_plan)
        .map_err(|failure| Box::new(adapter_failure(failure)))
}

pub(crate) fn valid_read_invocation(invocation: &ParsedInvocation, action: FitAction) -> bool {
    invocation.command == SuccessorCommand::Fit(action)
        && invocation.effect == EffectClass::Read
        && invocation.arguments.iter().all(|argument| {
            matches!(
                (&argument.name, &argument.value),
                (OptionName::Target, ParsedValue::RepositoryTarget(_))
            )
        })
        && invocation
            .arguments
            .iter()
            .filter(|argument| argument.name == OptionName::Target)
            .count()
            <= 1
}
