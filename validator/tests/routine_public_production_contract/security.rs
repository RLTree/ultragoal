use super::scenario::{Fixture, pass_node, prefix_route, tree};
use serde_json::Value;
use std::fs;
use std::os::unix::fs::symlink;

#[test]
#[ignore = "requires externally provisioned root-owned immutable current ultragoal binary"]
fn repository_cannot_turn_the_fixed_template_route_into_arbitrary_shell_authority() {
    Fixture::require_protected_binary();
    let fixture = Fixture::new(
        "arbitrary-shell-refusal",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        true,
    );
    fixture.rewrite_catalog_with_arbitrary_script();
    let before_root = tree(&fixture.root);
    let before_home = tree(&fixture.home);
    let before_status = fixture.status();
    let output = fixture.run();
    assert_diagnostic(&output, "successor_runtime_authority_required", &fixture);
    assert_eq!(tree(&fixture.root), before_root);
    assert_eq!(tree(&fixture.home), before_home);
    assert_eq!(fixture.status(), before_status);
    assert!(
        !fixture
            .root
            .join("target/routine/compile/false-pass")
            .exists()
    );
    assert_eq!(fs::read_dir(fixture.authority_root()).unwrap().count(), 0);
}

#[test]
fn legacy_manifest_cannot_bypass_the_pre_discovery_broker_refusal() {
    let fixture = Fixture::new(
        "legacy-manifest-refusal",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        true,
    );
    fixture.downgrade_manifest_schema();
    let before_root = tree(&fixture.root);
    let before_home = tree(&fixture.home);
    let before_status = fixture.status();
    let output = fixture.run();
    assert_diagnostic(&output, "successor_runtime_authority_required", &fixture);
    let diagnostic: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(diagnostic["cause"], "mediator-child-root-broker-required");
    assert_eq!(tree(&fixture.root), before_root);
    assert_eq!(tree(&fixture.home), before_home);
    assert_eq!(fixture.status(), before_status);
    assert_eq!(fs::read_dir(fixture.authority_root()).unwrap().count(), 0);
}

#[test]
fn missing_host_still_refuses_at_broker_before_any_workspace_write() {
    let fixture = Fixture::new(
        "missing-host",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        false,
    );
    let before_root = tree(&fixture.root);
    let before_home = tree(&fixture.home);
    let before_status = fixture.status();
    let output = fixture.run();
    assert_diagnostic(&output, "successor_runtime_authority_required", &fixture);
    assert_eq!(tree(&fixture.root), before_root);
    assert_eq!(tree(&fixture.home), before_home);
    assert_eq!(fixture.status(), before_status);
    assert!(
        !fixture
            .root
            .join("target/routine/compile/result.txt")
            .exists()
    );
}

#[test]
#[ignore = "requires externally provisioned root-owned immutable current ultragoal binary"]
fn host_lock_symlink_substitution_fails_closed() {
    Fixture::require_protected_binary();
    let fixture = Fixture::new(
        "lock-substitution",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        true,
    );
    fixture.substitute_lock_with_symlink();
    let before_root = tree(&fixture.root);
    let before_home = tree(&fixture.home);
    let output = fixture.run();
    assert_diagnostic(&output, "successor_runtime_authority_required", &fixture);
    assert_eq!(tree(&fixture.root), before_root);
    assert_eq!(tree(&fixture.home), before_home);
    assert_eq!(fs::read_dir(fixture.authority_root()).unwrap().count(), 0);
}

#[test]
fn target_symlink_substitution_cannot_bypass_the_pre_discovery_broker_refusal() {
    let target_fixture = Fixture::new(
        "target-substitution",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        true,
    );
    symlink(
        &target_fixture.root,
        target_fixture.root.join("aliased-target"),
    )
    .unwrap();
    let before_root = tree(&target_fixture.root);
    let before_home = tree(&target_fixture.home);
    let output =
        target_fixture.run_args(&["--json", "check", "routine", "--target", "aliased-target"]);
    assert_diagnostic(
        &output,
        "successor_runtime_authority_required",
        &target_fixture,
    );
    let diagnostic: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(diagnostic["cause"], "mediator-child-root-broker-required");
    assert_eq!(tree(&target_fixture.root), before_root);
    assert_eq!(tree(&target_fixture.home), before_home);
}

#[test]
fn catalog_digest_substitution_cannot_bypass_the_pre_discovery_broker_refusal() {
    let fixture = Fixture::new(
        "catalog-substitution",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        true,
    );
    fs::write(
        fixture.root.join("config/routines.json"),
        b"{\"schema_version\":\"substituted\"}",
    )
    .unwrap();
    let before_root = tree(&fixture.root);
    let before_home = tree(&fixture.home);
    let output = fixture.run();
    assert_diagnostic(&output, "successor_runtime_authority_required", &fixture);
    let diagnostic: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(diagnostic["cause"], "mediator-child-root-broker-required");
    assert_eq!(tree(&fixture.root), before_root);
    assert_eq!(tree(&fixture.home), before_home);
    assert_eq!(fs::read_dir(fixture.authority_root()).unwrap().count(), 0);
}

#[test]
fn help_parse_and_read_paths_never_open_routine_host_state() {
    let fixture = Fixture::new(
        "read-zero-write",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        true,
    );
    for args in [
        vec!["--json", "check", "routine", "--help"],
        vec!["--json", "check", "routine", "--unknown"],
        vec!["--json", "inspect", "context"],
    ] {
        let before_root = tree(&fixture.root);
        let before_home = tree(&fixture.home);
        let before_status = fixture.status();
        let output = fixture.run_args(&args);
        if args.contains(&"--unknown") {
            assert_ne!(output.status.code(), Some(0), "{output:?}");
        } else {
            assert_eq!(output.status.code(), Some(0), "{output:?}");
        }
        assert_eq!(tree(&fixture.root), before_root, "args={args:?}");
        assert_eq!(tree(&fixture.home), before_home, "args={args:?}");
        assert_eq!(fixture.status(), before_status, "args={args:?}");
        assert_eq!(fs::read_dir(fixture.authority_root()).unwrap().count(), 0);
    }
}

fn assert_diagnostic(output: &std::process::Output, id: &str, fixture: &Fixture) {
    assert_ne!(output.status.code(), Some(0), "{output:?}");
    assert!(output.stdout.is_empty(), "{output:?}");
    let value: Value = serde_json::from_slice(&output.stderr).unwrap_or_else(|error| {
        panic!(
            "diagnostic is not JSON: {error}; stderr={:?}",
            String::from_utf8_lossy(&output.stderr)
        )
    });
    assert_eq!(value["schema_version"], "HarnessDiagnostic-v1");
    assert_eq!(value["diagnostic_id"], id);
    let text = String::from_utf8_lossy(&output.stderr);
    assert!(!text.contains(fixture.root.to_str().unwrap()));
    assert!(!text.contains(fixture.home.to_str().unwrap()));
}
