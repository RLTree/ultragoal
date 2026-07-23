use super::*;

pub(crate) const MAX_PUBLIC_OUTPUT: usize = 16 * 1024 * 1024;
#[cfg(test)]
pub(crate) fn parse_public(raw: &[String]) -> Result<ParseOutcome, String> {
    parse_args(raw.iter().cloned()).map_err(|failure| failure.render())
}

pub(crate) fn run_public(root: &Path, outcome: ParseOutcome) -> Result<i32, String> {
    match outcome {
        ParseOutcome::Compatibility {
            command,
            output_mode,
        } => emit_compatibility(
            crate::cli::successor::compatibility::render_compatibility_guidance(
                command,
                output_mode,
            ),
        ),
        ParseOutcome::Help {
            target,
            output_mode,
        } => emit_text(render_help(target, output_mode)),
        ParseOutcome::Version(output_mode) => emit_text(version_text(output_mode)),
        ParseOutcome::Invocation(invocation) => run_invocation(root, invocation),
    }
}

pub(crate) fn run_invocation(root: &Path, invocation: ParsedInvocation) -> Result<i32, String> {
    let mode = invocation.output_mode;
    emit(execute_invocation(root, invocation), mode)
}

pub(crate) fn execute_invocation(root: &Path, invocation: ParsedInvocation) -> RuntimeOutcome {
    let home = std::env::var_os("HOME").map(PathBuf::from);
    execute_invocation_with_home(root, invocation, home.as_deref())
}

pub(crate) fn execute_invocation_with_home(
    root: &Path,
    invocation: ParsedInvocation,
    home: Option<&Path>,
) -> RuntimeOutcome {
    let Some(operation) = super::operation_binding::bind(&invocation) else {
        return crate::cli::successor::runtime::unavailable(&invocation);
    };
    if operation == super::operation_binding::PublicOperation::StrictCheck {
        return strict::execute(root, &invocation);
    }
    if operation == super::operation_binding::PublicOperation::RoutineCheck {
        return routine::execute(root, &invocation, home);
    }
    if operation == super::operation_binding::PublicOperation::EvaluationRun {
        return evaluation::run(&invocation);
    }
    if let Some(outcome) = package_dispatch::execute(root, &invocation, operation) {
        return outcome;
    }
    if invocation.effect != EffectClass::Read
        && operation != super::operation_binding::PublicOperation::FitApply
    {
        return crate::cli::successor::runtime::unavailable(&invocation);
    }
    let context_root = match invocation.command {
        SuccessorCommand::Fit(
            FitAction::Inspect | FitAction::Plan | FitAction::Apply | FitAction::Verify,
        ) => match fit::target_root(root, &invocation) {
            Ok(target) => target,
            Err(outcome) => return *outcome,
        },
        _ => root.to_path_buf(),
    };
    // Plan and apply intentionally share one effect-bound context identity. The
    // plan route still performs only reads and issues no mutation permit.
    let context_result = match invocation.command {
        SuccessorCommand::Fit(FitAction::Plan | FitAction::Apply) => {
            workspace_context(&context_root)
        }
        _ => read_context(&context_root),
    };
    let context = match context_result {
        Ok(context) => context,
        Err(()) => return context_unavailable(),
    };
    match invocation.command {
        SuccessorCommand::Inspect(InspectTarget::Context) => {
            public_context::project(&context, &invocation)
        }
        SuccessorCommand::Inspect(InspectTarget::Orchestration) => {
            orchestration::project(&context, &invocation)
        }
        SuccessorCommand::Inspect(InspectTarget::Inception) => {
            inception::project(&context, &invocation)
        }
        SuccessorCommand::Inspect(InspectTarget::Capabilities) => {
            capabilities::project(&context, &invocation, home)
        }
        SuccessorCommand::Inspect(InspectTarget::Inventory) => {
            match InventoryBuilder::new(&context).build() {
                Ok(inventory) if context.revalidate().is_ok() => inventory_outcome(&inventory),
                _ => inventory_unavailable(),
            }
        }
        SuccessorCommand::Observe(crate::cli::successor::ObserveAction::Query) => {
            observe::query_local(root, &context, &invocation)
        }
        SuccessorCommand::Eval(crate::cli::successor::command_contract::EvalAction::Audit) => {
            evaluation::audit(&context, &invocation)
        }
        SuccessorCommand::Fit(FitAction::Inspect) => fit::inspect(&context, &invocation),
        SuccessorCommand::Fit(FitAction::Plan) => fit::plan(&context, &invocation),
        SuccessorCommand::Fit(FitAction::Apply) => fit::apply(&context, &invocation, home),
        SuccessorCommand::Fit(FitAction::Verify) => fit::verify(&context, &invocation),
        SuccessorCommand::Migrate(crate::cli::successor::command_contract::MigrateAction::Plan) => {
            migration::plan(&context, &invocation)
        }
        SuccessorCommand::Diagnose if diagnose::requests_target(&invocation) => {
            diagnose::diagnose_routine(root, &context, &invocation, home)
        }
        SuccessorCommand::Diagnose => match InventoryBuilder::new(&context).build() {
            Ok(inventory) => match crate::state::derive_adopted(&context, &inventory) {
                Ok(state) => diagnose::diagnose_local(root, &context, state, &invocation),
                Err(_) => diagnose::diagnose_routine_or(
                    root,
                    &context,
                    &invocation,
                    home,
                    state_unavailable(),
                ),
            },
            Err(_) => diagnose::diagnose_routine_or(
                root,
                &context,
                &invocation,
                home,
                inventory_unavailable(),
            ),
        },
        SuccessorCommand::Inspect(
            InspectTarget::Summary | InspectTarget::Findings | InspectTarget::Claims,
        )
        | SuccessorCommand::Next => match InventoryBuilder::new(&context).build() {
            Ok(inventory) => match crate::state::derive_adopted(&context, &inventory) {
                Ok(state) => RuntimeSession::new(&context, Some(&state)).dispatch(&invocation),
                Err(_) => state_unavailable(),
            },
            Err(_) => inventory_unavailable(),
        },
        _ => crate::cli::successor::runtime::unavailable(&invocation),
    }
}

pub(crate) fn public_output_allowed(bytes: usize) -> bool {
    bytes <= MAX_PUBLIC_OUTPUT
}
