use super::*;

#[test]
fn repository_cannot_turn_the_fixed_template_route_into_arbitrary_shell_authority() {
    let mut fixture = Fixture::new(
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
    fixture.teardown_after_assertions();
}

#[test]
fn legacy_manifest_schema_is_rejected_before_effect() {
    let mut fixture = Fixture::new(
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
    assert_eq!(tree(&fixture.root), before_root);
    assert_eq!(tree(&fixture.home), before_home);
    assert_eq!(fixture.status(), before_status);
    assert_eq!(fs::read_dir(fixture.authority_root()).unwrap().count(), 0);
    fixture.teardown_after_assertions();
}

#[test]
fn normal_host_ancestry_bootstraps_private_authority_state() {
    let mut fixture = Fixture::new(
        "normal-host-ancestry",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        false,
    );
    let codex = fixture.home.join(".codex");
    fs::create_dir(&codex).unwrap();
    fs::set_permissions(&codex, fs::Permissions::from_mode(0o755)).unwrap();
    let output = fixture.run();
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(Fixture::value(&output)["status"], "executed");
    assert!(fixture.state_root().is_dir());
    assert_eq!(
        fs::metadata(fixture.state_root())
            .unwrap()
            .permissions()
            .mode()
            & 0o7777,
        0o700
    );
    fixture.teardown_after_assertions();
}

#[test]
fn writable_host_ancestry_refuses_without_effect() {
    let mut fixture = Fixture::new(
        "writable-host-ancestry",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        false,
    );
    let codex = fixture.home.join(".codex");
    fs::create_dir(&codex).unwrap();
    fs::set_permissions(&codex, fs::Permissions::from_mode(0o777)).unwrap();
    let before_root = tree(&fixture.root);
    let before_home = tree(&fixture.home);
    let output = fixture.run();
    assert_diagnostic(&output, "successor_runtime_authority_required", &fixture);
    assert_eq!(tree(&fixture.root), before_root);
    assert_eq!(tree(&fixture.home), before_home);
    assert!(!fixture.root.join("target/routine/compile").exists());
    fixture.teardown_after_assertions();
}

#[test]
fn private_authority_mode_is_revalidated_before_effect() {
    let mut fixture = Fixture::new(
        "private-authority-mode",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        true,
    );
    let private_root = fixture.home.join(".codex/state/harness-ultragoal");
    fs::set_permissions(&private_root, fs::Permissions::from_mode(0o755)).unwrap();
    let before_root = tree(&fixture.root);
    let before_home = tree(&fixture.home);
    let output = fixture.run();
    assert_diagnostic(&output, "successor_runtime_authority_required", &fixture);
    assert_eq!(tree(&fixture.root), before_root);
    assert_eq!(tree(&fixture.home), before_home);
    assert!(!fixture.root.join("target/routine/compile").exists());
    fixture.teardown_after_assertions();
}

#[test]
fn forged_short_staged_marker_refuses_without_publication() {
    let mut fixture = Fixture::new(
        "forged-staged-marker",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        false,
    );
    let stage = fixture
        .home
        .join(".codex/state/harness-ultragoal/.routine-public-bootstrap");
    for path in [
        fixture.home.join(".codex"),
        fixture.home.join(".codex/state"),
        fixture.home.join(".codex/state/harness-ultragoal"),
        stage.join("authority"),
        stage.join("adapter"),
    ] {
        fs::create_dir_all(&path).unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
    }
    let lock = stage.join("adapter/adapter.lock");
    fs::write(&lock, b"forged").unwrap();
    fs::set_permissions(&lock, fs::Permissions::from_mode(0o600)).unwrap();
    let before_root = tree(&fixture.root);
    let output = fixture.run();
    assert_diagnostic(&output, "successor_runtime_authority_required", &fixture);
    assert_eq!(tree(&fixture.root), before_root);
    assert_eq!(fs::read(&lock).unwrap(), b"forged");
    assert!(!fixture.state_root().exists());
    fixture.teardown_after_assertions();
}

#[test]
fn host_lock_symlink_substitution_fails_closed() {
    let mut fixture = Fixture::new(
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
    fixture.teardown_after_assertions();
}
