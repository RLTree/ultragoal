use super::*;
use crate::cli::successor::command_contract::{OptionName, PackageAction, ParsedValue};
use crate::distribution::{
    ConfinedRoot, ProductionPackageErrorId, ScopedFile, capture_product_package,
    verify_product_package,
};
use serde::Serialize;

#[derive(Serialize)]
struct PackageInventoryOutcome<'a> {
    schema_version: &'static str,
    candidate_id: &'a str,
    catalog_id: &'a str,
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
    let Some(output_path) = output_path(invocation) else {
        return invalid_invocation();
    };
    let catalog = match InventoryBuilder::new(source_context).build() {
        Ok(catalog) => catalog,
        Err(_) => return inventory_unavailable(),
    };
    let artifact = match capture_product_package(source_context, &catalog) {
        Ok(artifact) => artifact,
        Err(error) => return package_failure(error.id()),
    };
    if verify_product_package(&artifact, source_context, &catalog).is_err()
        || !public_output_allowed(artifact.snapshot().inventory().len())
    {
        return package_failure(ProductionPackageErrorId::InventoryMismatch);
    }
    let output = match ConfinedRoot::open_workspace(output_context)
        .and_then(|root| ScopedFile::new(root, output_path))
    {
        Ok(output) => output,
        Err(_) => return output_failure(),
    };
    if artifact
        .publish_inventory(source_context, &catalog, &output)
        .is_err()
    {
        return output_failure();
    }
    success(&artifact, output_path)
}

fn output_path(invocation: &ParsedInvocation) -> Option<&str> {
    match invocation {
        ParsedInvocation {
            command: SuccessorCommand::Package(PackageAction::Inventory),
            effect: EffectClass::WorkspaceWrite,
            arguments,
            ..
        } => match arguments.as_slice() {
            [argument] if argument.name == OptionName::Output => match &argument.value {
                ParsedValue::RelativePath(path) if inventory_output_allowed(path.as_str()) => {
                    Some(path.as_str())
                }
                _ => None,
            },
            _ => None,
        },
        _ => None,
    }
}

fn inventory_output_allowed(path: &str) -> bool {
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

fn success(
    artifact: &crate::distribution::ProductionPackageArtifact,
    output: &str,
) -> RuntimeOutcome {
    let snapshot = artifact.snapshot();
    let record = PackageInventoryOutcome {
        schema_version: "HarnessPackageInventoryOutcome-v1",
        candidate_id: artifact.candidate_id(),
        catalog_id: artifact.catalog_id(),
        inventory_sha256: snapshot.inventory_sha256(),
        inventory_byte_length: snapshot.inventory().len(),
        output,
        claim_ceiling: "package inventory written; package/install/runtime claims withheld",
    };
    match serde_json::to_vec(&record) {
        Ok(machine) if public_output_allowed(machine.len()) => RuntimeOutcome::payload(
            ExitClass::Success,
            machine,
            format!(
                "package inventory {} bytes={} output={output}",
                snapshot.inventory_sha256(),
                snapshot.inventory().len(),
            ),
        ),
        _ => output_failure(),
    }
}

fn invalid_invocation() -> RuntimeOutcome {
    diagnostic(
        DiagnosticId::UnexpectedArguments,
        ExitClass::InvalidInvocation,
        "package inventory requires exactly one bounded --output relative path",
        "invoke package inventory through the canonical successor grammar",
        "no package inventory or package claim is available",
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
        ProductionPackageErrorId::OutputFailed => "the package inventory output transaction failed",
    };
    diagnostic(
        DiagnosticId::InventoryUnavailable,
        ExitClass::UnsupportedCapability,
        cause,
        "repair the exact source, manifest, catalog, or candidate mismatch and retry",
        "package inventory and all package/install/runtime claims remain withheld",
    )
}

fn output_failure() -> RuntimeOutcome {
    diagnostic(
        DiagnosticId::DownstreamToolUnavailable,
        ExitClass::UnsupportedCapability,
        "the confined package inventory output could not settle atomically",
        "use an available safe workspace output path and retry on one stable candidate",
        "package inventory and all package/install/runtime claims remain withheld",
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
                affected_surface: "HCT-DISTRIBUTION package inventory",
                repair,
                effect: "workspace_write",
                rerun: "ultragoal --json package inventory --output target/ultragoal/package-inventory.json",
                ceiling,
            },
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::inventory_output_allowed;

    #[test]
    fn inventory_output_is_confined_to_disposable_product_namespace() {
        assert!(inventory_output_allowed(
            "target/ultragoal/package-inventory.json"
        ));
        assert!(inventory_output_allowed(
            "target/ultragoal/n04_inventory-1.json"
        ));
        for rejected in [
            ".git/HEAD",
            "plugin-manifest-draft.json",
            "target/ultragoal/nested/inventory.json",
            "target/ultragoal/inventory",
            "target/ultragoal/.json",
        ] {
            assert!(!inventory_output_allowed(rejected), "{rejected}");
        }
    }
}
