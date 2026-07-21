use super::*;
use crate::cli::successor::command_contract::{OptionName, PackageAction, ParsedValue};
use crate::distribution::{
    ProductionPackageErrorId, ReadOnlyWorkspace, capture_product_package, verify_product_package,
};
use serde::Serialize;

const PACKAGE_LIMIT: usize = 65 * 1024 * 1024;

pub(super) fn execute(
    root: &Path,
    invocation: &ParsedInvocation,
    operation: operation_binding::PublicOperation,
) -> Option<RuntimeOutcome> {
    match operation {
        operation_binding::PublicOperation::PackageBuild => Some(with_package_contexts(
            root,
            |source_context, output_context| {
                package_build::execute(source_context, output_context, invocation)
            },
        )),
        operation_binding::PublicOperation::PackageVerify => {
            Some(with_read_context(root, |context| {
                verify_current_package(context, invocation)
            }))
        }
        operation_binding::PublicOperation::PackageInventory => Some(with_package_contexts(
            root,
            |source_context, output_context| {
                package_inventory::execute(source_context, output_context, invocation)
            },
        )),
        operation_binding::PublicOperation::PackageInstallTest => Some(with_package_contexts(
            root,
            |source_context, output_context| {
                package_install_test::execute(source_context, output_context, invocation)
            },
        )),
        _ => None,
    }
}

fn with_read_context(
    root: &Path,
    execute: impl FnOnce(&LiveContext) -> RuntimeOutcome,
) -> RuntimeOutcome {
    match read_context(root) {
        Ok(context) => execute(&context),
        Err(()) => context_unavailable(),
    }
}

fn with_package_contexts(
    root: &Path,
    execute: impl FnOnce(&LiveContext, &LiveContext) -> RuntimeOutcome,
) -> RuntimeOutcome {
    match (read_context(root), workspace_context(root)) {
        (Ok(source_context), Ok(output_context)) => execute(&source_context, &output_context),
        (Err(()), _) | (_, Err(())) => context_unavailable(),
    }
}

#[derive(Serialize)]
struct PackageVerificationOutcome<'a> {
    schema_version: &'static str,
    candidate_id: &'a str,
    catalog_id: &'a str,
    package_sha256: &'a str,
    package_byte_length: usize,
    inventory_sha256: &'a str,
    inventory_byte_length: usize,
    claim_ceiling: &'static str,
}

fn verify_current_package(context: &LiveContext, invocation: &ParsedInvocation) -> RuntimeOutcome {
    let Some(input_path) = package_archive_input(invocation) else {
        return invalid_package_archive_input();
    };
    let catalog = match InventoryBuilder::new(context).build() {
        Ok(catalog) => catalog,
        Err(_) => return inventory_unavailable(),
    };
    let input = match ReadOnlyWorkspace::open(context)
        .and_then(|workspace| workspace.inspect_file(input_path, PACKAGE_LIMIT))
    {
        Ok(Some(bytes)) => bytes,
        _ => return package_verification_failure("the bounded package input is unavailable"),
    };
    let artifact = match capture_product_package(context, &catalog) {
        Ok(artifact) if verify_product_package(&artifact, context, &catalog).is_ok() => artifact,
        Ok(_) => return package_verification_failure("current source package verification failed"),
        Err(error) => return package_capture_failure(error.id()),
    };
    if input != artifact.snapshot().archive() {
        return package_verification_failure(
            "input bytes do not match the exact current-source package",
        );
    }
    if context.revalidate().is_err() || catalog.revalidate_identity().is_err() {
        return RuntimeOutcome::failure(
            ExitClass::ActionableFinding,
            Diagnostic::new(
                DiagnosticId::StaleContext,
                ExitClass::ActionableFinding,
                package_diagnostic(
                    "the candidate changed during package verification",
                    "rebuild the exact package from one stable candidate before retrying",
                    "same-candidate package evidence is invalidated",
                ),
            ),
        );
    }
    let snapshot = artifact.snapshot();
    let outcome = PackageVerificationOutcome {
        schema_version: "HarnessPackageVerificationOutcome-v1",
        candidate_id: artifact.candidate_id(),
        catalog_id: artifact.catalog_id(),
        package_sha256: snapshot.package_sha256(),
        package_byte_length: snapshot.archive().len(),
        inventory_sha256: snapshot.inventory_sha256(),
        inventory_byte_length: snapshot.inventory().len(),
        claim_ceiling: "exact current-source package bytes verified; install, cache, discovery, and runtime claims withheld",
    };
    match serde_json::to_vec(&outcome) {
        Ok(bytes) if public_output_allowed(bytes.len()) => RuntimeOutcome::payload(
            ExitClass::Success,
            bytes,
            format!("package verified {}", snapshot.package_sha256()),
        ),
        _ => package_verification_failure("package verification output exceeds the public bound"),
    }
}

fn package_archive_input(invocation: &ParsedInvocation) -> Option<&str> {
    match invocation {
        ParsedInvocation {
            command: SuccessorCommand::Package(PackageAction::Verify),
            effect: EffectClass::Read,
            arguments,
            ..
        } => match arguments.as_slice() {
            [argument] if argument.name == OptionName::Input => match &argument.value {
                ParsedValue::RelativePath(path) if package_archive_input_allowed(path.as_str()) => {
                    Some(path.as_str())
                }
                _ => None,
            },
            _ => None,
        },
        _ => None,
    }
}

pub(super) fn package_archive_input_allowed(path: &str) -> bool {
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

fn invalid_package_archive_input() -> RuntimeOutcome {
    RuntimeOutcome::failure(
        ExitClass::InvalidInvocation,
        Diagnostic::new(
            DiagnosticId::UnexpectedArguments,
            ExitClass::InvalidInvocation,
            package_diagnostic(
                "package verify requires one bounded target/ultragoal/*.hugpkg input",
                "supply the exact package archive through the canonical package grammar",
                "no package or dependent claim is available",
            ),
        ),
    )
}

fn package_capture_failure(id: ProductionPackageErrorId) -> RuntimeOutcome {
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
    package_verification_failure(cause)
}

fn package_verification_failure(cause: &'static str) -> RuntimeOutcome {
    RuntimeOutcome::failure(
        ExitClass::ActionableFinding,
        Diagnostic::new(
            DiagnosticId::InventoryUnavailable,
            ExitClass::ActionableFinding,
            package_diagnostic(
                cause,
                "rebuild the exact package from one stable candidate and retry verification",
                "package verification failed; install, cache, discovery, and runtime claims remain withheld",
            ),
        ),
    )
}

fn package_diagnostic(
    cause: &'static str,
    repair: &'static str,
    ceiling: &'static str,
) -> DiagnosticDetails {
    DiagnosticDetails {
        cause,
        affected_surface: "HCT-DISTRIBUTION package verify",
        repair,
        effect: "read",
        rerun: "ultragoal --json package verify --input target/ultragoal/package.hugpkg",
        ceiling,
    }
}
