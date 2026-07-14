use super::scenario::{Fixture, NodeSpec, pass_node, prefix_route, tree, wait_for_started};
use serde_json::Value;
use std::fs;
use std::os::unix::fs::symlink;

#[test]
fn repository_cannot_turn_the_fixed_template_route_into_arbitrary_shell_authority() {
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
fn missing_host_authority_refuses_before_any_workspace_write() {
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
fn stale_and_forged_reuse_are_rejected_before_ledger_transition() {
    for forged in [false, true] {
        let fixture = Fixture::new(
            if forged {
                "forged-reuse"
            } else {
                "stale-reuse"
            },
            &[pass_node("compile", &[])],
            &[prefix_route("route-src", "src", &["compile"])],
            true,
            true,
        );
        let first = fixture.run();
        assert_eq!(first.status.code(), Some(0), "{first:?}");
        if forged {
            fixture.forge_cache_artifact();
        } else {
            fixture.tamper_cache_field(
                "candidate_id",
                Value::String(format!("sha256:{}", "0".repeat(64))),
            );
        }
        let state_path = fixture.authority_root().join("routine-authority.state");
        let before_ledger = fs::read(&state_path).unwrap();
        let before_output =
            fs::read(fixture.root.join("target/routine/compile/result.txt")).unwrap();
        let output = fixture.run();
        assert_diagnostic(&output, "successor_runtime_authority_required", &fixture);
        assert_eq!(fs::read(state_path).unwrap(), before_ledger);
        assert_eq!(
            fs::read(fixture.root.join("target/routine/compile/result.txt")).unwrap(),
            before_output
        );
    }
}

#[test]
fn host_lock_symlink_and_target_symlink_substitution_fail_closed() {
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
        "successor_runtime_context_unavailable",
        &target_fixture,
    );
    assert_eq!(tree(&target_fixture.root), before_root);
    assert_eq!(tree(&target_fixture.home), before_home);
}

#[test]
fn catalog_digest_substitution_and_selected_source_race_withhold_success() {
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
    assert_diagnostic(&output, "successor_runtime_stale_context", &fixture);
    assert_eq!(tree(&fixture.root), before_root);
    assert_eq!(tree(&fixture.home), before_home);
    assert_eq!(fs::read_dir(fixture.authority_root()).unwrap().count(), 0);

    let node = NodeSpec {
        id: "compile",
        dependencies: &[],
        primary: "bash",
        fallback: None,
        action: "pass",
        delay_seconds: 2,
        read_sources: &["src/lib.rs"],
    };
    let race = Fixture::new(
        "selected-source-race",
        &[node],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        true,
    );
    let child = race.spawn();
    wait_for_started(&race.authority_root());
    race.mutate_selected_source(b"pub fn value() -> u8 { 99 }\n");
    let output = child.wait_with_output().unwrap();
    assert_diagnostic(&output, "successor_runtime_stale_context", &race);
    assert!(!race.cache_path().exists());
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
