use super::*;

#[test]
fn target_symlink_substitution_refuses_before_discovery() {
    let mut fixture = Fixture::new(
        "target-substitution",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        true,
    );
    symlink(&fixture.root, fixture.root.join("aliased-target")).unwrap();
    let before_root = tree(&fixture.root);
    let before_home = tree(&fixture.home);
    let output = fixture.run_args(&["--json", "check", "routine", "--target", "aliased-target"]);
    assert_diagnostic(&output, "successor_runtime_context_unavailable", &fixture);
    assert_eq!(tree(&fixture.root), before_root);
    assert_eq!(tree(&fixture.home), before_home);
    fixture.teardown_after_assertions();
}

#[test]
fn catalog_digest_substitution_refuses_before_effect() {
    let mut fixture = Fixture::new(
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
    fixture.teardown_after_assertions();
}

#[test]
fn help_parse_and_read_paths_never_open_routine_host_state() {
    let mut fixture = Fixture::new(
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
    fixture.teardown_after_assertions();
}
