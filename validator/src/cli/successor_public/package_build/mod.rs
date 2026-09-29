use super::*;
use crate::cli::successor::command_contract::{OptionName, PackageAction, ParsedValue};
use crate::distribution::{
    ConfinedRoot, ProductionPackageErrorId, ScopedFile, capture_product_package,
    capture_product_package_with_cli, verify_product_package,
};
use serde::Serialize;

#[derive(Serialize)]
struct PackageBuildOutcome<'a> {
    schema_version: &'static str,
    candidate_id: &'a str,
    catalog_id: &'a str,
    package_sha256: &'a str,
    package_byte_length: usize,
    inventory_sha256: &'a str,
    inventory_byte_length: usize,
    output: &'a str,
    claim_ceiling: &'static str,
}

pub(super) fn execute(
    source_context: &LiveContext,
    output_context: &LiveContext,
    invocation: &ParsedInvocation,
) -> RuntimeOutcome {
    let Some((output_path, cli_path)) = output_paths(invocation) else {
        return invalid_invocation();
    };
    let catalog = match InventoryBuilder::new(source_context).build() {
        Ok(catalog) => catalog,
        Err(error) => return inventory_failure(&error),
    };
    let source = match capture_product_package(source_context, &catalog) {
        Ok(artifact) => artifact,
        Err(error) => return package_failure(error.id()),
    };
    let payload = match super::package_cli_payload::from_workspace(
        output_context,
        cli_path,
        source.candidate_id(),
    ) {
        Some(payload) => payload,
        None => return package_failure(ProductionPackageErrorId::MembershipMismatch),
    };
    let artifact = match capture_product_package_with_cli(source_context, &catalog, payload) {
        Ok(artifact) => artifact,
        Err(error) => return package_failure(error.id()),
    };
    if verify_product_package(&artifact, source_context, &catalog).is_err()
        || !public_output_allowed(artifact.snapshot().archive().len())
    {
        return package_failure(ProductionPackageErrorId::ArchiveMismatch);
    }
    let output = match ConfinedRoot::open_workspace(output_context)
        .and_then(|root| ScopedFile::new(root, output_path))
    {
        Ok(output) => output,
        Err(_) => return output_failure(),
    };
    if artifact
        .publish_archive(source_context, &catalog, &output)
        .is_err()
    {
        return output_failure();
    }
    success(&artifact, output_path)
}

fn output_paths(invocation: &ParsedInvocation) -> Option<(&str, &str)> {
    match invocation {
        ParsedInvocation {
            command: SuccessorCommand::Package(PackageAction::Build),
            effect: EffectClass::WorkspaceWrite,
            arguments,
            ..
        } => {
            let output = arguments
                .iter()
                .find(|argument| argument.name == OptionName::Output)?;
            let cli = arguments
                .iter()
                .find(|argument| argument.name == OptionName::Cli)?;
            match (&output.value, &cli.value) {
                (ParsedValue::RelativePath(output), ParsedValue::RelativePath(cli))
                    if arguments.len() == 2
                        && output_allowed(output.as_str())
                        && super::package_cli_payload::allowed(cli.as_str()) =>
                {
                    Some((output.as_str(), cli.as_str()))
                }
                _ => None,
            }
        }
        _ => None,
    }
}

fn output_allowed(path: &str) -> bool {
    let Some(name) = path.strip_prefix("target/ultragoal/") else {
        return false;
    };
    let Some(stem) = name.strip_suffix(".hugpkg") else {
        return false;
    };
    !stem.is_empty()
        && !name.contains('/')
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}

fn success(
    artifact: &crate::distribution::ProductionPackageArtifact,
    output: &str,
) -> RuntimeOutcome {
    let snapshot = artifact.snapshot();
    let record = PackageBuildOutcome {
        schema_version: "HarnessPackageBuildOutcome-v1",
        candidate_id: artifact.candidate_id(),
        catalog_id: artifact.catalog_id(),
        package_sha256: snapshot.package_sha256(),
        package_byte_length: snapshot.archive().len(),
        inventory_sha256: snapshot.inventory_sha256(),
        inventory_byte_length: snapshot.inventory().len(),
        output,
        claim_ceiling: "package archive written; install, cache, discovery, and runtime claims withheld",
    };
    match serde_json::to_vec(&record) {
        Ok(machine) if public_output_allowed(machine.len()) => RuntimeOutcome::payload(
            ExitClass::Success,
            machine,
            format!(
                "package {} bytes={} output={output}",
                snapshot.package_sha256(),
                snapshot.archive().len(),
            ),
        ),
        _ => output_failure(),
    }
}

fn invalid_invocation() -> RuntimeOutcome {
    diagnostic(
        DiagnosticId::UnexpectedArguments,
        ExitClass::InvalidInvocation,
        "package build requires bounded --output and exact --cli relative paths",
        "build target/ultragoal/release/ultragoal, then write the package under target/ultragoal",
        "no package archive or dependent claim is available",
    )
}

fn package_failure(id: ProductionPackageErrorId) -> RuntimeOutcome {
    let cause = match id {
        ProductionPackageErrorId::ContextUnavailable => {
            "the candidate-bound package context changed during capture"
        }
        ProductionPackageErrorId::CatalogMismatch => {
            "the authority catalog does not bind the package context"
        }
        ProductionPackageErrorId::SourceUnavailable => {
            "canonical package source could not be captured"
        }
        ProductionPackageErrorId::ManifestMismatch => {
            "the plugin manifest does not match canonical package source"
        }
        ProductionPackageErrorId::MembershipMismatch => {
            "the generated package membership does not match canonical source"
        }
        ProductionPackageErrorId::ArchiveMismatch => {
            "the generated package archive did not independently verify"
        }
        ProductionPackageErrorId::InventoryMismatch => {
            "the generated package inventory did not independently verify"
        }
        ProductionPackageErrorId::OutputFailed => "the package output transaction failed",
    };
    diagnostic(
        DiagnosticId::InventoryUnavailable,
        ExitClass::UnsupportedCapability,
        cause,
        "repair the exact source, manifest, catalog, or candidate mismatch and retry",
        "package build and install, cache, discovery, and runtime claims remain withheld",
    )
}

fn output_failure() -> RuntimeOutcome {
    diagnostic(
        DiagnosticId::DownstreamToolUnavailable,
        ExitClass::UnsupportedCapability,
        "the confined package output could not settle atomically",
        "use an available target/ultragoal package path and retry on one stable candidate",
        "package output and all install, cache, discovery, and runtime claims remain withheld",
    )
}

fn diagnostic(
    id: DiagnosticId,
    exit: ExitClass,
    cause: &'static str,
    repair: &'static str,
    ceiling: &'static str,
) -> RuntimeOutcome {
    RuntimeOutcome::failure(
        exit,
        Diagnostic::new(
            id,
            exit,
            DiagnosticDetails {
                cause,
                affected_surface: "HCT-DISTRIBUTION package build",
                repair,
                effect: "workspace_write",
                rerun: "ultragoal --json package build --output target/ultragoal/package.hugpkg --cli target/ultragoal/release/ultragoal",
                ceiling,
            },
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::output_allowed;

    #[test]
    fn package_output_is_confined_to_disposable_product_namespace() {
        assert!(output_allowed("target/ultragoal/package.hugpkg"));
        assert!(output_allowed("target/ultragoal/current-source_1.hugpkg"));
        for rejected in [
            ".git/HEAD",
            "package.hugpkg",
            "target/ultragoal/nested/package.hugpkg",
            "target/ultragoal/package.zip",
            "target/ultragoal/.hugpkg",
        ] {
            assert!(!output_allowed(rejected), "{rejected}");
        }
        assert!(super::super::package_cli_payload::allowed(
            "target/ultragoal/release/ultragoal"
        ));
        assert!(!super::super::package_cli_payload::allowed(
            "validator/target/release/ultragoal"
        ));
    }
}
