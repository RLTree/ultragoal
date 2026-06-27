use crate::cli::control::plane::types::{ControlOperation, REQUIRED_COMMANDS};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

pub(crate) const RECEIPT_SCHEMA: &str = "harness-ultragoal.cli-control-plane-receipt.v1";

#[derive(Debug)]
pub(crate) struct ControlCommand {
    pub(crate) operation: ControlOperation,
    pub(crate) receipt: Option<PathBuf>,
}

pub(crate) fn parse(raw: &[String]) -> Result<Option<ControlCommand>, String> {
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
        _ => return Ok(None),
    };
    Ok(Some(ControlCommand {
        operation,
        receipt: opt_path(raw, "--receipt"),
    }))
}

pub(crate) fn run(root: &Path, command: &ControlCommand) -> Result<i32, String> {
    if let Some(path) = &command.receipt {
        path::validate_receipt_path(root, path, command.operation)?;
    }
    let receipt = receipt(root, command)?;
    let exit = i32::from(receipt.get("status").and_then(Value::as_str) != Some("pass"));
    if let Some(path) = &command.receipt {
        crate::json_boundary::write_json(path, &receipt)?;
        println!(
            "ultragoal-control {} operation={} receipt={}",
            receipt["status"],
            command.operation.id(),
            path.display()
        );
    } else {
        println!("{receipt}");
    }
    Ok(exit)
}

pub(crate) fn receipt(root: &Path, command: &ControlCommand) -> Result<Value, String> {
    let package_digest = crate::package::inventory::package_digest(root)?;
    let evidence_failures = proof::failures(root, command.operation);
    Ok(receipt_from_evidence(
        package_digest,
        command.operation,
        evidence_failures,
    ))
}

pub(crate) fn receipt_from_evidence(
    package_digest: String,
    operation: ControlOperation,
    mut evidence_failures: Vec<String>,
) -> Value {
    // No caller may turn an empty vector into update_goal authority until the CLI
    // can prove CLI, packet, and registry evidence as one transaction.
    if evidence_failures.is_empty() {
        evidence_failures.push("cli_control_plane_transactional_green_path_not_proven".to_string());
    }
    let blocked_claims = blocked_claims(operation);
    json!({
        "schema": RECEIPT_SCHEMA,
        "schema_version": "v1",
        "issuer": {
            "tool": "ultragoal",
            "authority": "cli_control_plane",
            "compatibility_binary": "ultragoal-validator",
            "self_law_state": "transition_only"
        },
        "generated_at": crate::audit::clock::now_iso(),
        "root": ".",
        "candidate_digest": package_digest,
        "operation": operation.id(),
        "status": "fail",
        "claim_ceiling": "withheld_or_blocked",
        "blocked_claim_classes": blocked_claims,
        "required_evidence": required_evidence(operation),
        "failure": proof::failure_value(operation, &evidence_failures),
        "command_surface": REQUIRED_COMMANDS,
        "notes": proof::notes(false, &evidence_failures)
    })
}

fn blocked_claims(operation: ControlOperation) -> Vec<&'static str> {
    let base = [
        "completion",
        "package_readiness",
        "review_readiness",
        "release_readiness",
        "update_goal_eligibility",
    ];
    let mut claims = base.to_vec();
    if matches!(
        operation,
        ControlOperation::RegistryProbe | ControlOperation::AppSurfaceProbe
    ) {
        claims.push("app_registry_or_reviewer_exposure");
    }
    claims
}

fn required_evidence(operation: ControlOperation) -> Vec<&'static str> {
    let mut out = vec![
        "current_red_fixture_report_status_pass",
        "coverage_100_no_uncovered_records",
        "current_cli_performance_pass",
        "current_final_packet_proof_pass",
        "live_registry_or_reviewer_exposure_same_surface_pass",
    ];
    if matches!(
        operation,
        ControlOperation::UpdateGoalEligibility | ControlOperation::SelfUpdateGoalEligibility
    ) {
        out.push("all_89_gates_and_100_stop_conditions_pass");
    }
    out
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

pub(crate) mod path;
pub(crate) mod proof;
pub(crate) mod receipt;
pub(crate) mod types;
