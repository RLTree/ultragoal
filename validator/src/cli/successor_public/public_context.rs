use crate::cli::successor::runtime::{
    Diagnostic, DiagnosticDetails, DiagnosticId, RuntimeOutcome, RuntimeSession,
};
use crate::cli::successor::{ExitClass, ParsedInvocation};
use crate::context::LiveContext;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::Read;
use std::path::Path;

const MAX_SELF_EXECUTABLE_BYTES: u64 = 128 * 1024 * 1024;

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

    let Some(runtime) = capture_runtime_path_identity() else {
        return projection_failure();
    };
    let machine = serde_json::to_vec(&json!({
        "schema_version": "HarnessPublicContext-v2",
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
        "runtime": runtime,
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

fn capture_runtime_path_identity() -> Option<serde_json::Value> {
    let path = std::env::current_exe().ok()?.canonicalize().ok()?;
    let identity = executable_identity(&path)?;
    let rebound = std::env::current_exe().ok()?.canonicalize().ok()?;
    let rebound_metadata = rebound.metadata().ok()?;
    if path != rebound
        || rebound_metadata.len() != identity.byte_length
        || rebound_metadata.modified().ok()? != identity.modified
    {
        return None;
    }
    Some(json!({
        "version": env!("CARGO_PKG_VERSION"),
        "executable_sha256": identity.sha256,
        "executable_byte_length": identity.byte_length,
        "self_bound": false,
    }))
}

#[derive(Eq, PartialEq)]
struct ExecutableIdentity {
    sha256: String,
    byte_length: u64,
    modified: std::time::SystemTime,
}

fn executable_identity(path: &Path) -> Option<ExecutableIdentity> {
    let before = path.metadata().ok()?;
    if !before.is_file() || before.len() == 0 || before.len() > MAX_SELF_EXECUTABLE_BYTES {
        return None;
    }
    let mut hasher = Sha256::new();
    let mut file = File::open(&path).ok()?;
    let mut bytes = 0_u64;
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer).ok()?;
        if read == 0 {
            break;
        }
        bytes = bytes.checked_add(read as u64)?;
        if bytes > MAX_SELF_EXECUTABLE_BYTES {
            return None;
        }
        hasher.update(&buffer[..read]);
    }
    let after = path.metadata().ok()?;
    if before.len() != bytes
        || after.len() != bytes
        || before.modified().ok()? != after.modified().ok()?
    {
        return None;
    }
    Some(ExecutableIdentity {
        sha256: format!("sha256:{:x}", hasher.finalize()),
        byte_length: bytes,
        modified: after.modified().ok()?,
    })
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
    use super::{capture_runtime_path_identity, root_id};

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

    #[test]
    fn path_occupant_identity_never_self_attests_the_running_image() {
        let runtime = capture_runtime_path_identity().expect("capture current executable path");
        assert_eq!(runtime["self_bound"], false);
        assert!(
            runtime["executable_sha256"]
                .as_str()
                .is_some_and(|digest| digest.starts_with("sha256:"))
        );
    }
}
