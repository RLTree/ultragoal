pub(super) fn assert_help(command: crate::Command) {
    assert!(matches!(command, crate::Command::Help));
}

pub(super) fn assert_routine(command: crate::Command) {
    assert!(matches!(command, crate::Command::Routine(_)));
}

pub(super) fn assert_product(command: crate::Command) {
    assert!(matches!(command, crate::Command::Product(_)));
}

pub(super) fn audit_parts(
    command: crate::Command,
) -> (Option<std::path::PathBuf>, std::path::PathBuf) {
    match command {
        crate::Command::Audit {
            target_repo,
            receipt,
            ..
        } => (target_repo, receipt),
        _ => panic!("target-repo audit must route to source-local target audit"),
    }
}

#[test]
fn routine_red_edges_reject_missing_help_and_script_surfaces() {
    let failures =
        crate::cli::routine::surface_failures("usage: ultragoal source audit --receipt x", "");
    for expected in [
        "routine_entrypoint_missing",
        "routine_help_not_navigable",
        "routine_leaf_only_validation_substitution",
        "routine_fit_repo_hidden",
        "routine_target_repo_hidden",
        "routine_scripts_check_claim_ceiling_missing",
    ] {
        assert!(failures.iter().any(|item| item == expected), "{expected}");
    }
    let fit_label_missing = crate::cli::routine::surface_failures(
        "Routine validation\nfit-repo prove\ntarget-repo audit\nTarget repo path\nunsupported claims\nroutine check\nline\nline\nline\nline\nline\nline",
        "routine check",
    );
    assert!(
        fit_label_missing
            .iter()
            .any(|item| item == "routine_fit_repo_hidden")
    );
    let target_label_missing = crate::cli::routine::surface_failures(
        "Routine validation\nfit-repo prove\nFirst plugin-activated repo path\ntarget-repo audit\nunsupported claims\nroutine check\nline\nline\nline\nline\nline\nline",
        "routine check",
    );
    assert!(
        target_label_missing
            .iter()
            .any(|item| item == "routine_target_repo_hidden")
    );
    let narrow_missing_claims = crate::cli::routine::surface_failures(
        "Routine validation\nfit-repo prove\nFirst plugin-activated repo path\ntarget-repo audit\nTarget repo path\nunsupported claims\nroutine check\nline\nline\nline\nline\nline",
        "narrow-helper ceiling",
    );
    assert!(
        narrow_missing_claims
            .iter()
            .any(|item| item == "routine_scripts_check_claim_ceiling_missing")
    );
}

#[test]
fn routine_route_assertions_fail_closed_for_wrong_routes() {
    for assertion in [
        || assert_help(crate::Command::PackageDigest),
        || assert_routine(crate::Command::PackageDigest),
        || assert_product(crate::Command::PackageDigest),
    ] {
        assert!(std::panic::catch_unwind(assertion).is_err());
    }
    assert!(std::panic::catch_unwind(|| audit_parts(crate::Command::PackageDigest)).is_err());
}
