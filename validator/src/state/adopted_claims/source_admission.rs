use super::lane_binding::DependencyIdentity;
use crate::context::{LiveContext, ReadSession, query_git};
use crate::state::StateError;

const GIT_OUTPUT_LIMIT: usize = 1024;

const OWNED_SOURCE_PATHS: [&str; 10] = [
    "validator/src/cli/successor_public/strict/claim_reconciliation_stage_adapter.rs",
    "validator/src/cli/successor_public/strict/failure_diagnostics.rs",
    "validator/src/cli/successor_public/strict/mod.rs",
    "validator/src/cli/successor_public/strict/tests.rs",
    "validator/src/state/adopted.rs",
    "validator/src/state/adopted_claims",
    "validator/src/state/adopted_registry.rs",
    "validator/src/state/mod.rs",
    "validator/src/state/tests/adopted",
    "validator/src/state/tests/mod.rs",
];

pub(super) fn verify(context: &LiveContext, source: &DependencyIdentity) -> Result<(), StateError> {
    context
        .revalidate()
        .map_err(|_| invalid("root-claim-source-context-stale"))?;
    let candidate = context.candidate();
    let head = candidate
        .head_commit
        .as_deref()
        .filter(|_| !candidate.dirty)
        .ok_or_else(|| invalid("root-claim-source-candidate-dirty"))?;
    let reads = context
        .begin_read_session()
        .map_err(|_| invalid("root-claim-source-context-stale"))?;
    verify_source(&reads, source, head)?;
    context
        .revalidate()
        .map_err(|_| invalid("root-claim-source-context-stale"))
}

fn verify_source(
    reads: &ReadSession,
    source: &DependencyIdentity,
    head: &str,
) -> Result<(), StateError> {
    let source_tree = git_text(
        reads,
        &[
            "rev-parse",
            "--verify",
            &format!("{}^{{tree}}", source.commit),
        ],
    )?;
    if source_tree != source.tree
        || git_success(
            reads,
            &["merge-base", "--is-ancestor", &source.commit, head],
            "root-claim-source-identity-invalid",
        )
        .is_err()
    {
        return Err(invalid("root-claim-source-identity-invalid"));
    }
    let mut args = vec![
        "diff",
        "--quiet",
        "--no-renames",
        "--no-ext-diff",
        "--no-textconv",
        &source.commit,
        head,
        "--",
    ];
    args.extend(OWNED_SOURCE_PATHS);
    if git_success(reads, &args, "root-claim-source-bytes-changed").is_err() {
        return Err(invalid("root-claim-source-bytes-changed"));
    }
    Ok(())
}

fn git_text(reads: &ReadSession, args: &[&str]) -> Result<String, StateError> {
    let stdout = git_bytes(reads, args, "root-claim-source-identity-unavailable")?;
    String::from_utf8(stdout)
        .map(|value| value.trim().to_owned())
        .map_err(|_| invalid("root-claim-source-identity-not-utf8"))
}

fn git_success(reads: &ReadSession, args: &[&str], code: &str) -> Result<(), StateError> {
    git_bytes(reads, args, code).map(|_| ())
}

fn git_bytes(reads: &ReadSession, args: &[&str], code: &str) -> Result<Vec<u8>, StateError> {
    let bytes = query_git(reads, args).map_err(|_| invalid(code))?;
    if bytes.len() > GIT_OUTPUT_LIMIT {
        return Err(invalid("root-claim-source-git-output-limit"));
    }
    Ok(bytes)
}

fn invalid(code: &str) -> StateError {
    StateError::InvalidCatalog(code.to_owned())
}

#[cfg(test)]
#[path = "source_admission_tests.rs"]
mod tests;
