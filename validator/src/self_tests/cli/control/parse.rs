use crate::cli::control::plane::operation::ControlOperation;
use crate::cli::control::plane::parse;

fn args(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

#[test]
fn parses_all_control_command_families() {
    let cases = [
        (
            &["law", "graph", "--strict"][..],
            ControlOperation::LawGraphStrict,
        ),
        (
            &["law", "check", "--gate", "89"][..],
            ControlOperation::LawCheckGate,
        ),
        (
            &["law", "check", "--all"][..],
            ControlOperation::LawCheckAll,
        ),
        (
            &["standards", "check", "--strict"][..],
            ControlOperation::StandardsCheckStrict,
        ),
        (
            &["source-obligations", "check", "--strict"][..],
            ControlOperation::SourceObligationsCheckStrict,
        ),
        (
            &["foundational-trace", "check", "--strict"][..],
            ControlOperation::FoundationalTraceCheckStrict,
        ),
        (
            &["namespace", "check", "--strict"][..],
            ControlOperation::NamespaceCheckStrict,
        ),
        (
            &["typed-boundaries", "check", "--strict"][..],
            ControlOperation::TypedBoundariesCheckStrict,
        ),
        (
            &["line-caps", "check", "--strict"][..],
            ControlOperation::LineCapsCheckStrict,
        ),
        (&["coverage", "prove"][..], ControlOperation::CoverageProve),
        (&["product", "init"][..], ControlOperation::ProductInit),
        (
            &["product", "prove-fitness"][..],
            ControlOperation::ProductProveFitness,
        ),
        (
            &["product", "prove-cohesion"][..],
            ControlOperation::ProductProveCohesion,
        ),
        (
            &["product", "prove-success"][..],
            ControlOperation::ProductProveSuccess,
        ),
        (
            &["product", "check-claims"][..],
            ControlOperation::ProductCheckClaims,
        ),
        (
            &["capability", "discover"][..],
            ControlOperation::CapabilityDiscover,
        ),
        (
            &["capability", "prove"][..],
            ControlOperation::CapabilityProve,
        ),
        (&["install", "audit"][..], ControlOperation::InstallAudit),
        (&["cache", "audit"][..], ControlOperation::CacheAudit),
        (&["registry", "probe"][..], ControlOperation::RegistryProbe),
        (
            &["app-surface", "probe"][..],
            ControlOperation::AppSurfaceProbe,
        ),
        (
            &["target-repo", "audit"][..],
            ControlOperation::TargetRepoAudit,
        ),
        (
            &["package", "inventory"][..],
            ControlOperation::PackageInventory,
        ),
        (&["package", "verify"][..], ControlOperation::PackageVerify),
        (&["packet", "build"][..], ControlOperation::PacketBuild),
        (&["packet", "verify"][..], ControlOperation::PacketVerify),
        (
            &["receipts", "verify"][..],
            ControlOperation::ReceiptsVerify,
        ),
        (&["fixtures", "red"][..], ControlOperation::FixturesRed),
        (&["fixtures", "green"][..], ControlOperation::FixturesGreen),
        (
            &["fixtures", "tamper"][..],
            ControlOperation::FixturesTamper,
        ),
        (&["fixtures", "all"][..], ControlOperation::FixturesAll),
        (
            &["clean-room", "rebuild"][..],
            ControlOperation::CleanRoomRebuild,
        ),
        (
            &["claim-ceiling", "compute"][..],
            ControlOperation::ClaimCeilingCompute,
        ),
        (&["explain", "failure-1"][..], ControlOperation::Explain),
        (
            &["failure", "capture"][..],
            ControlOperation::FailureCapture,
        ),
        (
            &["failure", "promote"][..],
            ControlOperation::FailurePromote,
        ),
        (
            &["issue", "check-lifecycle"][..],
            ControlOperation::IssueCheckLifecycle,
        ),
        (
            &["update-goal", "eligibility"][..],
            ControlOperation::UpdateGoalEligibility,
        ),
        (
            &["self", "audit", "--strict"][..],
            ControlOperation::SelfAuditStrict,
        ),
        (
            &["self", "law-graph", "--strict"][..],
            ControlOperation::SelfLawGraphStrict,
        ),
        (
            &["self", "fixtures", "red"][..],
            ControlOperation::SelfFixturesRed,
        ),
        (
            &["self", "fixtures", "green"][..],
            ControlOperation::SelfFixturesGreen,
        ),
        (
            &["self", "fixtures", "tamper"][..],
            ControlOperation::SelfFixturesTamper,
        ),
        (
            &["self", "update-goal", "eligibility"][..],
            ControlOperation::SelfUpdateGoalEligibility,
        ),
    ];
    for (raw, operation) in cases {
        let parsed = parse(&args(raw)).expect("control command");
        assert_eq!(parsed.operation, operation, "raw={raw:?}");
    }
    assert!(parse(&args(&["law", "graph"])).is_none());
    assert!(parse(&args(&["unknown"])).is_none());
}

#[test]
fn parses_package_surface_audit_roots() {
    let install = parse(&args(&[
        "install",
        "audit",
        "--receipt",
        "validation_artifacts/cli/install-audit-receipt.json",
        "--installed-root",
        "target/installed",
    ]))
    .expect("install audit");
    assert_eq!(install.operation, ControlOperation::InstallAudit);
    assert_eq!(
        install.surface_root.as_deref(),
        Some(std::path::Path::new("target/installed"))
    );

    let cache = parse(&args(&[
        "cache",
        "audit",
        "--receipt",
        "validation_artifacts/cli/cache-audit-receipt.json",
        "--cache-root",
        "target/cache",
    ]))
    .expect("cache audit");
    assert_eq!(cache.operation, ControlOperation::CacheAudit);
    assert_eq!(
        cache.surface_root.as_deref(),
        Some(std::path::Path::new("target/cache"))
    );
}

#[test]
fn agent_authority_roots_require_one_complete_explicit_pair() {
    let complete = parse(&args(&[
        "registry",
        "probe",
        "--agent-package-root",
        "target/package",
        "--agent-project-root",
        "target/project",
    ]))
    .expect("complete agent authority roots");
    let roots = complete
        .agent_authority_roots
        .expect("agent authority roots");
    assert_eq!(roots.package, std::path::Path::new("target/package"));
    assert_eq!(roots.project, std::path::Path::new("target/project"));
    assert!(
        parse(&args(&[
            "registry",
            "probe",
            "--agent-package-root",
            "target/package",
        ]))
        .is_none()
    );
    assert!(
        parse(&args(&[
            "registry",
            "probe",
            "--agent-project-root",
            "target/project",
        ]))
        .is_none()
    );
}
