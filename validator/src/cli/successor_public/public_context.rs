use crate::cli::successor::runtime::{
    Diagnostic, DiagnosticDetails, DiagnosticId, RuntimeOutcome, RuntimeSession,
};
use crate::cli::successor::{ExitClass, ParsedInvocation};
use crate::context::LiveContext;
use serde_json::json;
use sha2::{Digest, Sha256};

pub(super) fn project(context: &LiveContext, invocation: &ParsedInvocation) -> RuntimeOutcome {
    project_with_limit(context, invocation, super::MAX_PUBLIC_OUTPUT)
}

pub(super) fn project_with_limit(
    context: &LiveContext,
    invocation: &ParsedInvocation,
    maximum_output: usize,
) -> RuntimeOutcome {
    if !invocation.arguments.is_empty() || context.revalidate().is_err() {
        return RuntimeSession::new(context, None).dispatch(invocation);
    }

    let machine = serde_json::to_vec(&json!({
        "schema_version": "HarnessPublicContext-v1",
        "context_id": context.context_id(),
        "roots": {
            "repository_root_id": root_id(
                context.context_id(),
                "repository",
                &context.roots().repository_root,
            ),
            "worktree_root_id": root_id(
                context.context_id(),
                "worktree",
                &context.roots().worktree_root,
            ),
        },
        "candidate": context.candidate(),
        "configuration": context.configuration(),
        "capabilities": context.capabilities(),
        "permissions": context.permissions(),
        "effect": context.effect(),
        "selected_inputs": context.selected_inputs(),
    }));

    if context.revalidate().is_err() {
        return RuntimeSession::new(context, None).dispatch(invocation);
    }

    match machine {
        Ok(machine) if machine.len() <= maximum_output => RuntimeOutcome::payload(
            ExitClass::Success,
            machine,
            format!(
                "context {} dirty={}",
                context.context_id(),
                context.candidate().dirty
            ),
        ),
        _ => projection_failure(),
    }
}

fn projection_failure() -> RuntimeOutcome {
    let class = ExitClass::InternalFailure;
    RuntimeOutcome::failure(
        class,
        Diagnostic::new(
            DiagnosticId::ProjectionFailed,
            class,
            DiagnosticDetails {
                cause: "the bounded public context projection could not be produced",
                affected_surface: "public context output",
                repair: "repair the public projection without exposing the internal LiveContext payload",
                effect: "read",
                rerun: "ultragoal --json inspect context",
                ceiling: "public context and dependent claims remain withheld",
            },
        ),
    )
}

fn root_id(context_id: &str, role: &str, absolute_root: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"harness-public-root-id-v1\0");
    hasher.update(context_id.as_bytes());
    hasher.update([0]);
    hasher.update(role.as_bytes());
    hasher.update([0]);
    hasher.update(absolute_root.as_bytes());
    format!("sha256:{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::root_id;

    #[test]
    fn root_ids_are_context_and_role_bound_without_path_echo() {
        let path = "/private/operator/canary";
        let repository = root_id("sha256:context-a", "repository", path);
        let worktree = root_id("sha256:context-a", "worktree", path);
        let other_context = root_id("sha256:context-b", "repository", path);

        assert!(repository.starts_with("sha256:"));
        assert_eq!(repository.len(), 71);
        assert_ne!(repository, worktree);
        assert_ne!(repository, other_context);
        assert!(!repository.contains(path));
    }
}
