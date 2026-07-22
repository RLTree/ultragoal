use super::*;
use crate::cli::successor::command_contract::{OptionName, PackageAction, ParsedValue};
use crate::distribution::{
    ProductionPackageErrorId, ReadOnlyWorkspace, capture_product_package,
    capture_product_package_with_cli, verify_product_package,
};
use serde::Serialize;

const PACKAGE_LIMIT: usize = 65 * 1024 * 1024;

#[derive(Serialize)]
struct Outcome<'a> {
    schema_version: &'static str,
    candidate_id: &'a str,
    catalog_id: &'a str,
    package_sha256: &'a str,
    package_byte_length: usize,
    inventory_sha256: &'a str,
    inventory_byte_length: usize,
    claim_ceiling: &'static str,
}

pub(super) fn execute(context: &LiveContext, invocation: &ParsedInvocation) -> RuntimeOutcome {
    let Some((input_path, cli_path)) = input(invocation) else {
        return invalid_input();
    };
    let catalog = match InventoryBuilder::new(context).build() {
        Ok(catalog) => catalog,
        Err(error) => return inventory_failure(&error),
    };
    let input = match ReadOnlyWorkspace::open(context)
        .and_then(|workspace| workspace.inspect_file(input_path, PACKAGE_LIMIT))
    {
        Ok(Some(bytes)) => bytes,
        _ => return failure("the bounded package input is unavailable"),
    };
    let source = match capture_product_package(context, &catalog) {
        Ok(artifact) => artifact,
        Err(error) => return capture_failure(error.id()),
    };
    let payload = match super::package_cli_payload::from_read_only(
        context,
        cli_path,
        source.candidate_id(),
    ) {
        Some(payload) => payload,
        None => return failure("candidate CLI payload is unavailable or not native"),
    };
    let artifact = match capture_product_package_with_cli(context, &catalog, payload) {
        Ok(artifact) if verify_product_package(&artifact, context, &catalog).is_ok() => artifact,
        Ok(_) => return failure("current source package verification failed"),
        Err(error) => return capture_failure(error.id()),
    };
    if input != artifact.snapshot().archive() {
        return failure("input bytes do not match the exact current-source package");
    }
    if context.revalidate().is_err() || catalog.revalidate_identity().is_err() {
        return RuntimeOutcome::failure(
            ExitClass::ActionableFinding,
            Diagnostic::new(
                DiagnosticId::StaleContext,
                ExitClass::ActionableFinding,
                diagnostic(
                    "the candidate changed during package verification",
                    "rebuild the exact package from one stable candidate before retrying",
                    "same-candidate package evidence is invalidated",
                ),
            ),
        );
    }
    let snapshot = artifact.snapshot();
    let outcome = Outcome {
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
        _ => failure("package verification output exceeds the public bound"),
    }
}

fn input(invocation: &ParsedInvocation) -> Option<(&str, &str)> {
    let ParsedInvocation {
        command: SuccessorCommand::Package(PackageAction::Verify),
        effect: EffectClass::Read,
        arguments,
        ..
    } = invocation
    else {
        return None;
    };
    let input = arguments
        .iter()
        .find(|argument| argument.name == OptionName::Input)?;
    let cli = arguments
        .iter()
        .find(|argument| argument.name == OptionName::Cli)?;
    match (&input.value, &cli.value) {
        (ParsedValue::RelativePath(input), ParsedValue::RelativePath(cli))
            if arguments.len() == 2
                && super::package_dispatch::package_archive_input_allowed(input.as_str())
                && super::package_cli_payload::allowed(cli.as_str()) =>
        {
            Some((input.as_str(), cli.as_str()))
        }
        _ => None,
    }
}

fn invalid_input() -> RuntimeOutcome {
    RuntimeOutcome::failure(
        ExitClass::InvalidInvocation,
        Diagnostic::new(
            DiagnosticId::UnexpectedArguments,
            ExitClass::InvalidInvocation,
            diagnostic(
                "package verify requires one bounded archive and exact candidate CLI input",
                "supply the exact package archive and target/ultragoal/release/ultragoal",
                "no package or dependent claim is available",
            ),
        ),
    )
}

fn capture_failure(id: ProductionPackageErrorId) -> RuntimeOutcome {
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
    failure(cause)
}

fn failure(cause: &'static str) -> RuntimeOutcome {
    RuntimeOutcome::failure(
        ExitClass::ActionableFinding,
        Diagnostic::new(
            DiagnosticId::InventoryUnavailable,
            ExitClass::ActionableFinding,
            diagnostic(
                cause,
                "rebuild the exact package from one stable candidate and retry verification",
                "package verification failed; install, cache, discovery, and runtime claims remain withheld",
            ),
        ),
    )
}

fn diagnostic(
    cause: &'static str,
    repair: &'static str,
    ceiling: &'static str,
) -> DiagnosticDetails {
    DiagnosticDetails {
        cause,
        affected_surface: "HCT-DISTRIBUTION package verify",
        repair,
        effect: "read",
        rerun: "ultragoal --json package verify --input target/ultragoal/package.hugpkg --cli target/ultragoal/release/ultragoal",
        ceiling,
    }
}
