use crate::cli::successor::runtime::{Diagnostic, DiagnosticId, RuntimeOutcome, RuntimeSession};
use crate::cli::successor::{
    EffectClass, ExitClass, FitAction, InspectTarget, OutputMode, ParseOutcome, ParsedInvocation,
    SuccessorCommand, parse_args, render_help, version_text,
};
use crate::context::{BuildRequest, LiveContext};
use crate::inventory::{
    ADOPTED_HANDOFF_DIGEST_CONFIG_KEY, ADOPTED_HANDOFF_MANIFEST_SHA256, AuthorityCatalog,
    InventoryBuilder,
};
use std::io::{self, Write};
use std::path::Path;

const MAX_PUBLIC_OUTPUT: usize = 16 * 1024 * 1024;

pub(crate) fn parse_public(raw: &[String]) -> Result<ParseOutcome, String> {
    parse_args(raw.iter().cloned()).map_err(|failure| failure.render())
}

pub(crate) fn run_public(root: &Path, outcome: ParseOutcome) -> Result<i32, String> {
    match outcome {
        ParseOutcome::Help {
            target,
            output_mode,
        } => emit_text(render_help(target, output_mode)),
        ParseOutcome::Version(output_mode) => emit_text(version_text(output_mode)),
        ParseOutcome::Invocation(invocation) => run_invocation(root, invocation),
    }
}

fn run_invocation(root: &Path, invocation: ParsedInvocation) -> Result<i32, String> {
    let mode = invocation.output_mode;
    emit(execute_invocation(root, invocation), mode)
}

fn execute_invocation(root: &Path, invocation: ParsedInvocation) -> RuntimeOutcome {
    if invocation.effect != EffectClass::Read {
        return crate::cli::successor::runtime::unavailable(&invocation);
    }
    let context_root = match invocation.command {
        SuccessorCommand::Fit(FitAction::Inspect | FitAction::Plan | FitAction::Verify) => {
            match fit::target_root(root, &invocation) {
                Ok(target) => target,
                Err(outcome) => return outcome,
            }
        }
        _ => root.to_path_buf(),
    };
    let context = match read_context(&context_root) {
        Ok(context) => context,
        Err(()) => return context_unavailable(),
    };
    match invocation.command {
        SuccessorCommand::Inspect(InspectTarget::Context) => {
            public_context::project(&context, &invocation)
        }
        SuccessorCommand::Inspect(InspectTarget::Capabilities) => {
            RuntimeSession::new(&context, None).dispatch(&invocation)
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
        SuccessorCommand::Fit(FitAction::Inspect) => fit::inspect(&context, &invocation),
        SuccessorCommand::Fit(FitAction::Plan) => fit::plan(&context, &invocation),
        SuccessorCommand::Fit(FitAction::Verify) => fit::verify(&context, &invocation),
        SuccessorCommand::Diagnose => match InventoryBuilder::new(&context).build() {
            Ok(inventory) => match crate::state::derive_adopted(&context, &inventory) {
                Ok(state) => diagnose::diagnose_local(root, &context, &state, &invocation),
                Err(_) => state_unavailable(),
            },
            Err(_) => inventory_unavailable(),
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

fn read_context(root: &Path) -> Result<LiveContext, ()> {
    LiveContext::build(
        BuildRequest::new(root)
            .with_effect(EffectClass::Read)
            .bind_non_secret_configuration(
                ADOPTED_HANDOFF_DIGEST_CONFIG_KEY,
                ADOPTED_HANDOFF_MANIFEST_SHA256,
            ),
    )
    .map_err(|_| ())
}

fn inventory_outcome(inventory: &AuthorityCatalog) -> RuntimeOutcome {
    let exit = if inventory.has_error_findings() {
        ExitClass::ActionableFinding
    } else {
        ExitClass::Success
    };
    match inventory.to_canonical_json() {
        Ok(machine) if public_output_allowed(machine.len()) => RuntimeOutcome::payload(
            exit,
            machine,
            format!(
                "inventory {} entries={} findings={}",
                inventory.catalog_id(),
                inventory.entries().len(),
                inventory.findings().len()
            ),
        ),
        Ok(_) => failure(
            DiagnosticId::InventoryUnavailable,
            "the canonical authority inventory exceeds the bounded public output",
            "authority inventory output",
            "narrow the repository surface or repair unbounded authority discovery before retrying",
            "inventory and dependent claims remain unavailable",
        ),
        Err(_) => inventory_unavailable(),
    }
}

fn public_output_allowed(bytes: usize) -> bool {
    bytes <= MAX_PUBLIC_OUTPUT
}

fn context_unavailable() -> RuntimeOutcome {
    failure(
        DiagnosticId::ContextUnavailable,
        "a current read-only LiveContext could not be constructed",
        "live repository context",
        "verify the root and retry against one stable Git candidate",
        "context and dependent claims remain unavailable",
    )
}

fn inventory_unavailable() -> RuntimeOutcome {
    failure(
        DiagnosticId::InventoryUnavailable,
        "the canonical authority inventory could not be derived from the current context",
        "authority inventory",
        "repair the adopted registry or concurrent candidate mutation and rerun inspection",
        "inventory and dependent claims remain unavailable",
    )
}

fn state_unavailable() -> RuntimeOutcome {
    failure(
        DiagnosticId::StateUnavailable,
        "the canonical typed state graph could not be derived from current authority inputs",
        "typed product state",
        "repair the current inventory or adopted state policy and rerun inspection",
        "state and completion claims remain unavailable",
    )
}

fn failure(
    id: DiagnosticId,
    cause: &'static str,
    surface: &'static str,
    repair: &'static str,
    ceiling: &'static str,
) -> RuntimeOutcome {
    RuntimeOutcome::failure(
        ExitClass::UnsupportedCapability,
        Diagnostic::new(
            id,
            ExitClass::UnsupportedCapability,
            cause,
            surface,
            repair,
            "read",
            "ultragoal --json inspect context",
            ceiling,
        ),
    )
}

fn emit(outcome: RuntimeOutcome, mode: OutputMode) -> Result<i32, String> {
    let streams = outcome.render(mode);
    write_all(io::stdout().lock(), &streams.stdout)?;
    write_all(io::stderr().lock(), &streams.stderr)?;
    Ok(streams.exit_code)
}

fn emit_text(text: String) -> Result<i32, String> {
    let mut bytes = text.into_bytes();
    if !bytes.ends_with(b"\n") {
        bytes.push(b'\n');
    }
    write_all(io::stdout().lock(), &bytes)?;
    Ok(0)
}

fn write_all(mut output: impl Write, bytes: &[u8]) -> Result<(), String> {
    output
        .write_all(bytes)
        .map_err(|_| "successor runtime output failed".to_owned())
}

mod diagnose;
mod fit;
mod local_store;
mod observe;
mod public_context;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

#[cfg(test)]
#[path = "diagnose_tests.rs"]
mod diagnose_tests;

#[cfg(test)]
#[path = "diagnose_boundary_tests.rs"]
mod diagnose_boundary_tests;

#[cfg(test)]
mod test_support;
