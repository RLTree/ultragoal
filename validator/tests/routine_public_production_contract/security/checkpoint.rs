use super::*;

#[test]
fn malformed_stale_or_partially_published_checkpoint_fails_closed() {
    for (label, field) in [
        ("stale-checkpoint-head", "authenticated_ledger_head"),
        ("foreign-checkpoint-binding", "context_id"),
    ] {
        let mut fixture = Fixture::new(
            label,
            &[pass_node("compile", &[])],
            &[prefix_route("route-src", "src", &["compile"])],
            true,
            true,
        );
        let interrupted = fixture.run_args(&[
            "--json",
            "check",
            "routine",
            "--interrupt-after",
            "reservation",
        ]);
        assert_eq!(interrupted.status.code(), Some(1), "{interrupted:?}");
        let continuation = Fixture::value(&interrupted)["continuation"]
            .as_str()
            .unwrap()
            .to_owned();
        let checkpoint = fixture.checkpoint_path();
        let mut value: Value = serde_json::from_slice(&fs::read(&checkpoint).unwrap()).unwrap();
        value[field] = Value::String(format!("sha256:{field}"));
        fs::write(&checkpoint, serde_json::to_vec(&value).unwrap()).unwrap();
        let before = tree(&fixture.root);
        let output = fixture.run_args(&[
            "--json",
            "check",
            "routine",
            "--continuation",
            &continuation,
        ]);
        assert_diagnostic(&output, "successor_runtime_authority_required", &fixture);
        assert_eq!(tree(&fixture.root), before);
        assert!(!fixture.root.join("target/routine/compile").exists());
        fixture.teardown_after_assertions();
    }

    let mut fixture = Fixture::new(
        "truncated-checkpoint",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        true,
    );
    let interrupted = fixture.run_args(&[
        "--json",
        "check",
        "routine",
        "--interrupt-after",
        "reservation",
    ]);
    assert_eq!(interrupted.status.code(), Some(1), "{interrupted:?}");
    fs::write(fixture.checkpoint_path(), b"{\"schema_version\":").unwrap();
    let before = tree(&fixture.root);
    let output = fixture.run_args(&[
        "--json",
        "check",
        "routine",
        "--continuation",
        "routine-cont-truncated",
    ]);
    assert_diagnostic(&output, "successor_runtime_authority_required", &fixture);
    assert_eq!(tree(&fixture.root), before);
    assert!(!fixture.root.join("target/routine/compile").exists());
    fixture.teardown_after_assertions();
}

#[test]
fn interrupted_checkpoint_stage_residue_refuses_without_replacement_or_effect() {
    let mut fixture = Fixture::new(
        "checkpoint-stage-residue",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        true,
    );
    let stage = fixture.checkpoint_stage_path();
    fs::write(&stage, b"partial-checkpoint").unwrap();
    fs::set_permissions(&stage, fs::Permissions::from_mode(0o600)).unwrap();
    let before_root = tree(&fixture.root);
    let before_home = tree(&fixture.home);
    let output = fixture.run();
    assert_diagnostic(&output, "successor_runtime_authority_required", &fixture);
    assert_eq!(tree(&fixture.root), before_root);
    assert_eq!(tree(&fixture.home), before_home);
    assert_eq!(fs::read(stage).unwrap(), b"partial-checkpoint");
    assert!(!fixture.root.join("target/routine/compile").exists());
    fixture.teardown_after_assertions();
}

#[test]
fn canonical_continuation_directory_rejects_unknown_and_staged_entries() {
    for (label, name) in [
        ("unknown-continuation-entry", "unexpected"),
        (
            "canonical-continuation-stage-residue",
            ".routine-continuation-0000000000000000000000000000000000000000000000000000000000000000.next",
        ),
    ] {
        let mut fixture = Fixture::new(
            label,
            &[pass_node("compile", &[])],
            &[prefix_route("route-src", "src", &["compile"])],
            true,
            true,
        );
        fs::create_dir_all(fixture.continuations_root()).unwrap();
        fs::write(fixture.continuations_root().join(name), b"partial").unwrap();
        fs::set_permissions(
            fixture.continuations_root().join(name),
            fs::Permissions::from_mode(0o600),
        )
        .unwrap();
        let before_root = tree(&fixture.root);
        let before_home = tree(&fixture.home);
        let output = fixture.run();
        assert_diagnostic(&output, "successor_runtime_authority_required", &fixture);
        assert_eq!(tree(&fixture.root), before_root);
        assert_eq!(tree(&fixture.home), before_home);
        fixture.teardown_after_assertions();
    }
}

#[test]
fn canonical_continuation_symlink_and_hardlink_substitution_fail_closed() {
    for (label, hard_link) in [
        ("canonical-continuation-symlink", false),
        ("canonical-continuation-hardlink", true),
    ] {
        let mut fixture = Fixture::new(
            label,
            &[pass_node("compile", &[])],
            &[prefix_route("route-src", "src", &["compile"])],
            true,
            true,
        );
        let interrupted = fixture.run_args(&[
            "--json",
            "check",
            "routine",
            "--interrupt-after",
            "reservation",
        ]);
        assert_eq!(interrupted.status.code(), Some(1), "{interrupted:?}");
        let continuation = Fixture::value(&interrupted)["continuation"]
            .as_str()
            .unwrap()
            .to_owned();
        let checkpoint = fixture.checkpoint_path();
        let substitute = fixture.container.join("continuation-substitute.json");
        fs::copy(&checkpoint, &substitute).unwrap();
        fs::remove_file(&checkpoint).unwrap();
        if hard_link {
            fs::hard_link(&substitute, &checkpoint).unwrap();
        } else {
            symlink(&substitute, &checkpoint).unwrap();
        }
        let before_root = tree(&fixture.root);
        let output = fixture.run_args(&[
            "--json",
            "check",
            "routine",
            "--continuation",
            &continuation,
        ]);
        assert_diagnostic(&output, "successor_runtime_authority_required", &fixture);
        assert_eq!(tree(&fixture.root), before_root);
        assert!(!fixture.root.join("target/routine/compile").exists());
        fixture.teardown_after_assertions();
    }
}
