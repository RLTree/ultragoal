use super::*;
use crate::cli::successor::command_contract::{OptionName, PackageAction, ParsedValue};
use crate::distribution::{
    ConfinedRoot, ProductionPackageArtifact, ScopedFile, capture_product_package,
    verify_product_package,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
mod isolated_observation;
mod isolated_transaction;
mod temporary_root;
#[cfg(test)]
mod tests;

const PACKAGE_LIMIT: usize = 65 * 1024 * 1024;
const OUTPUT_LIMIT: usize = 1024 * 1024;

#[derive(Serialize)]
struct InstallTestOutcome<'a> {
    schema_version: &'static str,
    candidate_id: &'a str,
    catalog_id: &'a str,
    package_sha256: &'a str,
    installed_observation_sha256: &'a str,
    marketplace_source_tree_sha256: &'a str,
    cache_observation_sha256: &'a str,
    marketplace_observation_sha256: &'a str,
    runtime_observation_sha256: &'a str,
    journey_binding_sha256: &'a str,
    discovery_status: &'static str,
    output: &'a str,
    claim_ceiling: &'static str,
}

pub(super) fn execute(
    source_context: &LiveContext,
    output_context: &LiveContext,
    invocation: &ParsedInvocation,
) -> RuntimeOutcome {
    let Some((input_path, output_path)) = invocation_paths(invocation) else {
        return invalid_invocation();
    };
    let catalog = match InventoryBuilder::new(source_context).build() {
        Ok(catalog) => catalog,
        Err(_) => return transaction_failure("authority inventory is unavailable"),
    };
    let artifact = match capture_product_package(source_context, &catalog) {
        Ok(artifact) if verify_product_package(&artifact, source_context, &catalog).is_ok() => {
            artifact
        }
        _ => return transaction_failure("current-source package capture failed"),
    };
    if !input_matches(output_context, input_path, &artifact) {
        return transaction_failure("input package is not the exact current-source archive");
    }
    let observation = match isolated_transaction::execute(source_context, &catalog, &artifact) {
        Ok(observation) => observation,
        Err(cause) => return transaction_failure(cause),
    };
    let record = InstallTestOutcome {
        schema_version: "HarnessPackageInstallTestOutcome-v1",
        candidate_id: artifact.candidate_id(),
        catalog_id: artifact.catalog_id(),
        package_sha256: artifact.snapshot().package_sha256(),
        installed_observation_sha256: observation.installed_observation_sha256(),
        marketplace_source_tree_sha256: observation.marketplace_source_tree_sha256(),
        cache_observation_sha256: observation.cache_observation_sha256(),
        marketplace_observation_sha256: observation.marketplace_observation_sha256(),
        runtime_observation_sha256: observation.runtime_observation_sha256(),
        journey_binding_sha256: observation.journey_binding_sha256(),
        discovery_status: "pending-fresh-codex-task",
        output: output_path,
        claim_ceiling: "isolated source/archive, public plugin-install record, marketplace, cache, and runtime object verified; app-registry, Codex discovery, and installed-product claims withheld",
    };
    publish_outcome(output_context, output_path, &record)
}

fn invocation_paths(invocation: &ParsedInvocation) -> Option<(&str, &str)> {
    let ParsedInvocation {
        command: SuccessorCommand::Package(PackageAction::InstallTest),
        effect: EffectClass::WorkspaceWrite,
        arguments,
        ..
    } = invocation
    else {
        return None;
    };
    if arguments.len() != 2 {
        return None;
    }
    let input = arguments
        .iter()
        .find(|argument| argument.name == OptionName::Input);
    let output = arguments
        .iter()
        .find(|argument| argument.name == OptionName::Output);
    match (
        input.map(|argument| &argument.value),
        output.map(|argument| &argument.value),
    ) {
        (Some(ParsedValue::RelativePath(input)), Some(ParsedValue::RelativePath(output)))
            if super::package_dispatch::package_archive_input_allowed(input.as_str())
                && output_allowed(output.as_str()) =>
        {
            Some((input.as_str(), output.as_str()))
        }
        _ => None,
    }
}

fn input_matches(
    context: &LiveContext,
    input_path: &str,
    artifact: &ProductionPackageArtifact,
) -> bool {
    ConfinedRoot::open_workspace(context)
        .and_then(|root| ScopedFile::new(root, input_path))
        .and_then(|file| file.inspect(PACKAGE_LIMIT))
        .ok()
        .flatten()
        .is_some_and(|bytes| bytes == artifact.snapshot().archive())
}

fn publish_outcome(
    context: &LiveContext,
    output_path: &str,
    record: &InstallTestOutcome<'_>,
) -> RuntimeOutcome {
    let Ok(bytes) = serde_json::to_vec(record) else {
        return transaction_failure("install-test outcome could not be encoded");
    };
    let Ok(root) = ConfinedRoot::open_workspace(context) else {
        return transaction_failure("install-test outcome publication failed");
    };
    let Ok(file) = ScopedFile::new(root, output_path) else {
        return transaction_failure("install-test outcome publication failed");
    };
    let Ok(current) = file.inspect(OUTPUT_LIMIT) else {
        return transaction_failure("install-test outcome publication failed");
    };
    let expected = current.as_deref().map(digest);
    if file.apply(expected.as_deref(), Some(&bytes)).ok() != Some(true)
        || file.inspect(OUTPUT_LIMIT).ok().flatten().as_deref() != Some(bytes.as_slice())
    {
        return transaction_failure("install-test outcome publication failed");
    }
    RuntimeOutcome::payload(
        ExitClass::Success,
        bytes,
        format!("package install-test output={output_path}"),
    )
}

fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn transaction_failure(cause: &'static str) -> RuntimeOutcome {
    RuntimeOutcome::failure(
        ExitClass::UnsupportedCapability,
        Diagnostic::new(
            DiagnosticId::DownstreamToolUnavailable,
            ExitClass::UnsupportedCapability,
            DiagnosticDetails {
                cause,
                affected_surface: "HCT-DISTRIBUTION package install-test",
                repair: "repair the exact package or isolated transaction failure and retry",
                effect: "workspace_write plus disposable isolated host effects",
                rerun: "ultragoal --json package install-test --input target/ultragoal/package.hugpkg --output target/ultragoal/install-test.json",
                ceiling: "Codex discovery, installed product, readiness, and release claims remain withheld",
            },
        ),
    )
}

fn output_allowed(path: &str) -> bool {
    let Some(name) = path.strip_prefix("target/ultragoal/") else {
        return false;
    };
    let Some(stem) = name.strip_suffix(".json") else {
        return false;
    };
    !stem.is_empty()
        && !name.contains('/')
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}

fn invalid_invocation() -> RuntimeOutcome {
    RuntimeOutcome::failure(
        ExitClass::InvalidInvocation,
        Diagnostic::new(
            DiagnosticId::UnexpectedArguments,
            ExitClass::InvalidInvocation,
            DiagnosticDetails {
                cause: "package install-test requires bounded --input and disposable --output relative paths",
                affected_surface: "HCT-DISTRIBUTION package install-test",
                repair: "supply a package input and target/ultragoal/*.json output",
                effect: "none",
                rerun: "ultragoal --json package install-test --input target/ultragoal/package.hugpkg --output target/ultragoal/install-test.json",
                ceiling: "no package, install, cache, discovery, or runtime claim is available",
            },
        ),
    )
}
