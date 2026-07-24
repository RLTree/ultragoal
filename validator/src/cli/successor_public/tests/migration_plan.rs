use super::*;
use crate::cli::successor::ExitClass;
use crate::cli::successor_public::strict;
use std::process::Command;

#[test]
fn current_source_migration_plan_is_empty_and_without_writes() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let before_tree = strict::zero_write_guard::capture(root).unwrap();
    let before = git_status(root);
    let request = invocation(&[
        "--json",
        "migrate",
        "plan",
        "--registry",
        "migration/authority-routes.json",
    ]);
    let outcome = execute_invocation(root, request);
    assert_eq!(outcome.exit_class, ExitClass::Success);
    let projection: serde_json::Value =
        serde_json::from_slice(outcome.machine_payload.as_ref().unwrap()).unwrap();
    assert_eq!(
        projection["schema_version"],
        "ProductMigrationPlanProjection-v2"
    );
    assert_eq!(projection["item_count"], 0);
    assert_eq!(projection["effect_count"], 0);
    assert_eq!(projection["items"], serde_json::json!([]));
    assert_eq!(projection["effects"], serde_json::json!([]));
    assert!(
        !String::from_utf8_lossy(outcome.machine_payload.as_ref().unwrap())
            .contains(&root.to_string_lossy().to_string())
    );
    let default = execute_invocation(root, invocation(&["--json", "migrate", "plan"]));
    assert_eq!(default.machine_payload, outcome.machine_payload);
    assert_eq!(git_status(root), before);
    assert_eq!(
        strict::zero_write_guard::capture(root).unwrap(),
        before_tree
    );
}

#[test]
fn migration_plan_rejects_noncanonical_registry_argument_before_projection() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let invocation = invocation(&[
        "--json",
        "migrate",
        "plan",
        "--registry",
        "migration/other.json",
    ]);
    let outcome = execute_invocation(root, invocation);
    assert_eq!(outcome.exit_class, ExitClass::InvalidInvocation);
    assert!(outcome.machine_payload.is_none());
    assert!(!outcome
        .render(crate::cli::successor::OutputMode::Json)
        .stderr
        .windows("migration/other.json".len())
        .any(|window| window == b"migration/other.json"));
}

#[test]
fn migration_verify_is_public_read_only_and_bound_to_the_current_authority_catalog() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let before_tree = strict::zero_write_guard::capture(root).unwrap();
    let before = git_status(root);
    let outcome = execute_invocation(root, invocation(&["--json", "migrate", "verify"]));
    assert_eq!(outcome.exit_class, ExitClass::Success);
    let projection: serde_json::Value =
        serde_json::from_slice(outcome.machine_payload.as_ref().unwrap()).unwrap();
    assert_eq!(
        projection["schema_version"],
        "ProductMigrationVerification-v1"
    );
    assert!(projection["catalog_id"]
        .as_str()
        .is_some_and(|value| value.starts_with("sha256:")));
    assert_eq!(projection["authority_error_codes"], serde_json::json!([]));
    assert_eq!(
        projection["plan"]["schema_version"],
        "ProductMigrationPlanProjection-v2"
    );
    assert_eq!(projection["plan"]["item_count"], 0);
    assert_eq!(projection["plan"]["effect_count"], 0);
    assert_eq!(git_status(root), before);
    assert_eq!(
        strict::zero_write_guard::capture(root).unwrap(),
        before_tree
    );
}

#[test]
fn migration_verify_rejects_noncanonical_registry_argument_before_verification() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let invocation = invocation(&[
        "--json",
        "migrate",
        "verify",
        "--registry",
        "migration/other.json",
    ]);
    let outcome = execute_invocation(root, invocation);
    assert_eq!(outcome.exit_class, ExitClass::InvalidInvocation);
    assert!(outcome.machine_payload.is_none());
}

fn invocation(arguments: &[&str]) -> ParsedInvocation {
    match parse_args(arguments.iter().copied()).unwrap() {
        ParseOutcome::Invocation(invocation) => invocation,
        _ => panic!("expected invocation"),
    }
}

fn git_status(root: &Path) -> Vec<u8> {
    let output = Command::new("git")
        .args(["status", "--porcelain=v1", "-z", "--untracked-files=all"])
        .env("GIT_OPTIONAL_LOCKS", "0")
        .current_dir(root)
        .output()
        .unwrap();
    assert!(output.status.success());
    output.stdout
}
