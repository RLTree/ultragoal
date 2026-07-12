use super::diagnostics::{Diagnostic, DiagnosticId, RuntimeOutcome};
use super::failures::{
    delegated, downstream, effect_mismatch, projection_failure, stale_context,
    state_context_mismatch, state_unavailable, unexpected_arguments,
};
use super::{StateDisposition, StateProjection, StateView};
use crate::context::LiveContext;
use serde_json::json;

use super::super::{
    ExitClass, InspectTarget, OptionName, ParsedInvocation, ParsedValue, SuccessorCommand,
};

const MAX_RUNTIME_OUTPUT: usize = 16 * 1024 * 1024;

pub(crate) struct RuntimeSession<'a> {
    context: &'a LiveContext,
    state: Option<&'a dyn StateView>,
}

impl<'a> RuntimeSession<'a> {
    pub const fn new(context: &'a LiveContext, state: Option<&'a dyn StateView>) -> Self {
        Self { context, state }
    }

    pub fn dispatch(&self, invocation: &ParsedInvocation) -> RuntimeOutcome {
        if self.context.revalidate().is_err() {
            return stale_context();
        }
        let Some(descriptor) = super::super::catalog()
            .iter()
            .find(|descriptor| descriptor.command == invocation.command)
        else {
            return projection_failure();
        };
        if descriptor.effect != invocation.effect
            || self.context.effect().authorize(invocation.effect).is_err()
        {
            return effect_mismatch();
        }
        if self
            .state
            .is_some_and(|state| state.context_id() != self.context.context_id())
        {
            return state_context_mismatch();
        }
        let outcome = match invocation.command {
            SuccessorCommand::Inspect(InspectTarget::Context) => {
                if invocation.arguments.is_empty() {
                    self.context_projection()
                } else {
                    unexpected_arguments()
                }
            }
            SuccessorCommand::Inspect(InspectTarget::Capabilities) => {
                if invocation.arguments.is_empty() {
                    self.capability_projection()
                } else {
                    unexpected_arguments()
                }
            }
            SuccessorCommand::Inspect(InspectTarget::Summary) => {
                self.state_projection(invocation, StateProjection::Summary, "summary")
            }
            SuccessorCommand::Inspect(InspectTarget::Findings) => {
                self.state_projection(invocation, StateProjection::Findings, "findings")
            }
            SuccessorCommand::Inspect(InspectTarget::Claims) => {
                self.state_projection(invocation, StateProjection::Claims, "claims")
            }
            SuccessorCommand::Next => {
                self.state_projection(invocation, StateProjection::Next, "next")
            }
            SuccessorCommand::Diagnose => self.diagnose(invocation),
            SuccessorCommand::Inspect(InspectTarget::Inventory) => {
                delegated(invocation.effect, "HCT-INVENTORY", "N02-INVENTORY")
            }
            _ => {
                let (tool, node) = downstream(invocation.command);
                delegated(invocation.effect, tool, node)
            }
        };
        if self.context.revalidate().is_err() {
            stale_context()
        } else {
            outcome
        }
    }

    fn context_projection(&self) -> RuntimeOutcome {
        match self.context.to_canonical_json() {
            Ok(machine) if valid_payload(&machine) => RuntimeOutcome::payload(
                ExitClass::Success,
                machine,
                format!(
                    "context {} dirty={}",
                    self.context.context_id(),
                    self.context.candidate().dirty
                ),
            ),
            _ => projection_failure(),
        }
    }

    fn capability_projection(&self) -> RuntimeOutcome {
        let available = self
            .context
            .capabilities()
            .tools
            .iter()
            .filter(|tool| tool.available)
            .count();
        let machine = serde_json::to_vec(&json!({
            "schema_version": "HarnessCapabilities-v1",
            "context_id": self.context.context_id(),
            "path_search_sha256": self.context.capabilities().path_search_sha256,
            "tools": self.context.capabilities().tools,
        }));
        match machine {
            Ok(machine) if valid_payload(&machine) => RuntimeOutcome::payload(
                ExitClass::Success,
                machine,
                format!(
                    "capabilities context={} available={} total={}",
                    self.context.context_id(),
                    available,
                    self.context.capabilities().tools.len()
                ),
            ),
            _ => projection_failure(),
        }
    }

    fn state_projection(
        &self,
        invocation: &ParsedInvocation,
        projection: StateProjection<'_>,
        label: &'static str,
    ) -> RuntimeOutcome {
        if !invocation.arguments.is_empty() {
            return unexpected_arguments();
        }
        let Some(state) = self.current_state() else {
            return state_unavailable();
        };
        let exit = state_exit(state, projection);
        match state.project(projection) {
            Ok(Some(machine)) if valid_payload(&machine) => RuntimeOutcome::payload(
                exit,
                machine,
                format!(
                    "state {} projection={} findings={}",
                    state.state_id(),
                    label,
                    state.finding_count()
                ),
            ),
            _ => projection_failure(),
        }
    }

    fn diagnose(&self, invocation: &ParsedInvocation) -> RuntimeOutcome {
        let finding = match invocation.arguments.as_slice() {
            [] => None,
            [argument]
                if argument.name == OptionName::Finding
                    && matches!(argument.value, ParsedValue::Identifier(_)) =>
            {
                let ParsedValue::Identifier(value) = &argument.value else {
                    unreachable!()
                };
                Some(value.as_str())
            }
            _ => return unexpected_arguments(),
        };
        let Some(state) = self.current_state() else {
            return state_unavailable();
        };
        match state.project(StateProjection::Diagnose(finding)) {
            Ok(Some(machine)) if valid_payload(&machine) => RuntimeOutcome::payload(
                state_exit(state, StateProjection::Diagnose(finding)),
                machine,
                format!(
                    "state {} projection=diagnose findings={}",
                    state.state_id(),
                    state.finding_count()
                ),
            ),
            Ok(None) => RuntimeOutcome::failure(
                ExitClass::ActionableFinding,
                Diagnostic::new(
                    DiagnosticId::FindingNotPresent,
                    ExitClass::ActionableFinding,
                    "requested finding is not present in the current state graph",
                    "diagnose",
                    "re-run inspect findings and select a current finding identifier",
                    "read",
                    "ultragoal --json inspect findings",
                    "runtime claim remains current-state-only",
                ),
            ),
            _ => projection_failure(),
        }
    }

    fn current_state(&self) -> Option<&dyn StateView> {
        self.state
    }
}

fn state_exit(state: &dyn StateView, projection: StateProjection<'_>) -> ExitClass {
    if matches!(projection, StateProjection::Next) {
        return match state.disposition() {
            StateDisposition::NoAction => ExitClass::Success,
            StateDisposition::Action => ExitClass::ActionableFinding,
            StateDisposition::AuthorityRequest | StateDisposition::NoLegalRoute => {
                ExitClass::BlockedAuthority
            }
        };
    }
    if state.finding_count() == 0 {
        ExitClass::Success
    } else {
        ExitClass::ActionableFinding
    }
}

fn valid_payload(bytes: &[u8]) -> bool {
    if bytes.len() > MAX_RUNTIME_OUTPUT {
        return false;
    }
    serde_json::from_slice::<serde_json::Value>(bytes)
        .ok()
        .and_then(|value| value.get("schema_version")?.as_str().map(str::to_owned))
        .is_some_and(|schema| schema.ends_with("-v1"))
}
