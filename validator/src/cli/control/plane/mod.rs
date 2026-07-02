use crate::cli::control::plane::types::ControlOperation;
use serde_json::Value;
use std::path::{Path, PathBuf};

pub(crate) const RECEIPT_SCHEMA: &str = "harness-ultragoal.cli-control-plane-receipt.v1";

#[derive(Debug)]
pub(crate) struct ControlCommand {
    pub(crate) operation: ControlOperation,
    pub(crate) receipt: Option<PathBuf>,
    pub(crate) surface_root: Option<PathBuf>,
}

pub(crate) fn parse(raw: &[String]) -> Option<ControlCommand> {
    let operation = match raw {
        [a, b, rest @ ..] if a == "law" && b == "graph" && has_flag(rest, "--strict") => {
            ControlOperation::LawGraphStrict
        }
        [a, b, rest @ ..] if a == "law" && b == "check" && has_flag(rest, "--gate") => {
            ControlOperation::LawCheckGate
        }
        [a, b, rest @ ..] if a == "law" && b == "check" && has_flag(rest, "--all") => {
            ControlOperation::LawCheckAll
        }
        [a, b, rest @ ..] if a == "standards" && b == "check" && has_flag(rest, "--strict") => {
            ControlOperation::StandardsCheckStrict
        }
        [a, b, rest @ ..]
            if a == "source-obligations" && b == "check" && has_flag(rest, "--strict") =>
        {
            ControlOperation::SourceObligationsCheckStrict
        }
        [a, b, rest @ ..]
            if a == "foundational-trace" && b == "check" && has_flag(rest, "--strict") =>
        {
            ControlOperation::FoundationalTraceCheckStrict
        }
        [a, b, rest @ ..] if a == "namespace" && b == "check" && has_flag(rest, "--strict") => {
            ControlOperation::NamespaceCheckStrict
        }
        [a, b, rest @ ..]
            if a == "typed-boundaries" && b == "check" && has_flag(rest, "--strict") =>
        {
            ControlOperation::TypedBoundariesCheckStrict
        }
        [a, b, rest @ ..] if a == "line-caps" && b == "check" && has_flag(rest, "--strict") => {
            ControlOperation::LineCapsCheckStrict
        }
        [a, b, ..] if a == "coverage" && b == "prove" => ControlOperation::CoverageProve,
        [a, b, ..] if a == "product" && b == "init" => ControlOperation::ProductInit,
        [a, b, ..] if a == "product" && b == "prove-fitness" => {
            ControlOperation::ProductProveFitness
        }
        [a, b, ..] if a == "product" && b == "prove-cohesion" => {
            ControlOperation::ProductProveCohesion
        }
        [a, b, ..] if a == "product" && b == "prove-success" => {
            ControlOperation::ProductProveSuccess
        }
        [a, b, ..] if a == "product" && b == "check-claims" => ControlOperation::ProductCheckClaims,
        [a, b, ..] if a == "capability" && b == "discover" => ControlOperation::CapabilityDiscover,
        [a, b, ..] if a == "capability" && b == "prove" => ControlOperation::CapabilityProve,
        [a, b, ..] if a == "install" && b == "audit" => ControlOperation::InstallAudit,
        [a, b, ..] if a == "cache" && b == "audit" => ControlOperation::CacheAudit,
        [a, b, ..] if a == "registry" && b == "probe" => ControlOperation::RegistryProbe,
        [a, b, ..] if a == "app-surface" && b == "probe" => ControlOperation::AppSurfaceProbe,
        [a, b, ..] if a == "target-repo" && b == "audit" => ControlOperation::TargetRepoAudit,
        [a, b, ..] if a == "package" && b == "inventory" => ControlOperation::PackageInventory,
        [a, b, ..] if a == "package" && b == "verify" => ControlOperation::PackageVerify,
        [a, b, ..] if a == "packet" && b == "build" => ControlOperation::PacketBuild,
        [a, b, ..] if a == "packet" && b == "verify" => ControlOperation::PacketVerify,
        [a, b, ..] if a == "receipts" && b == "verify" => ControlOperation::ReceiptsVerify,
        [a, b, ..] if a == "fixtures" && b == "red" => ControlOperation::FixturesRed,
        [a, b, ..] if a == "fixtures" && b == "green" => ControlOperation::FixturesGreen,
        [a, b, ..] if a == "fixtures" && b == "tamper" => ControlOperation::FixturesTamper,
        [a, b, ..] if a == "fixtures" && b == "all" => ControlOperation::FixturesAll,
        [a, b, ..] if a == "clean-room" && b == "rebuild" => ControlOperation::CleanRoomRebuild,
        [a, b, ..] if a == "claim-ceiling" && b == "compute" => {
            ControlOperation::ClaimCeilingCompute
        }
        [a, ..] if a == "explain" => ControlOperation::Explain,
        [a, b, ..] if a == "failure" && b == "capture" => ControlOperation::FailureCapture,
        [a, b, ..] if a == "failure" && b == "promote" => ControlOperation::FailurePromote,
        [a, b, ..] if a == "issue" && b == "check-lifecycle" => {
            ControlOperation::IssueCheckLifecycle
        }
        [a, b, ..] if a == "update-goal" && b == "eligibility" => {
            ControlOperation::UpdateGoalEligibility
        }
        [a, b, rest @ ..] if a == "self" && b == "audit" && has_flag(rest, "--strict") => {
            ControlOperation::SelfAuditStrict
        }
        [a, b, rest @ ..] if a == "self" && b == "law-graph" && has_flag(rest, "--strict") => {
            ControlOperation::SelfLawGraphStrict
        }
        [a, b, c, ..] if a == "self" && b == "fixtures" && c == "red" => {
            ControlOperation::SelfFixturesRed
        }
        [a, b, c, ..] if a == "self" && b == "fixtures" && c == "green" => {
            ControlOperation::SelfFixturesGreen
        }
        [a, b, c, ..] if a == "self" && b == "fixtures" && c == "tamper" => {
            ControlOperation::SelfFixturesTamper
        }
        [a, b, c, ..] if a == "self" && b == "update-goal" && c == "eligibility" => {
            ControlOperation::SelfUpdateGoalEligibility
        }
        _ => return None,
    };
    Some(ControlCommand {
        operation,
        receipt: opt_path(raw, "--receipt"),
        surface_root: surface_root(raw),
    })
}

pub(crate) fn run(root: &Path, command: &ControlCommand) -> Result<i32, String> {
    let started = std::time::Instant::now();
    if let Some(path) = &command.receipt {
        path::validate_receipt_path(root, path, command.operation)?;
    }
    if surface::supports(command.operation) {
        return surface::run(root, command);
    }
    let package_digest = crate::package::inventory::package_digest(root)?;
    registry::mint_fail_closed_if_needed(root, command.operation, &package_digest)?;
    let mut receipt = receipt(root, command)?;
    registry::telemetry::attach(root, command, &mut receipt, started)?;
    let exit = i32::from(receipt.get("status").and_then(Value::as_str) != Some("pass"));
    if let Some(path) = &command.receipt {
        crate::json_boundary::write_json(path, &receipt)?;
        registry::stdout::print(path, &receipt);
    } else {
        println!("{receipt}");
    }
    Ok(exit)
}

pub(crate) fn receipt(root: &Path, command: &ControlCommand) -> Result<Value, String> {
    let package_digest = crate::package::inventory::package_digest(root)?;
    let evidence_failures = proof::failures(root, command.operation);
    Ok(emit::receipt_from_production_evidence(
        root,
        package_digest,
        command.operation,
        evidence_failures,
    ))
}

fn has_flag(args: &[String], flag: &str) -> bool {
    args.iter().any(|arg| arg == flag)
}

fn opt_path(args: &[String], key: &str) -> Option<PathBuf> {
    args.iter()
        .position(|arg| arg == key)
        .and_then(|index| args.get(index + 1))
        .map(PathBuf::from)
}

fn surface_root(args: &[String]) -> Option<PathBuf> {
    opt_path(args, "--surface-root")
        .or_else(|| opt_path(args, "--installed-root"))
        .or_else(|| opt_path(args, "--cache-root"))
}

pub(crate) mod emit;
pub(crate) mod evidence;
pub(crate) mod path;
pub(crate) mod proof;
pub(crate) mod receipt;
pub(crate) mod registry;
pub(crate) mod surface;
pub(crate) mod transactional;
pub(crate) mod types;
#[cfg(test)]
pub(crate) use emit::receipt_from_evidence;
